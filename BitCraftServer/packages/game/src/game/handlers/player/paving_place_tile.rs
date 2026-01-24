use crate::game::claim_helper;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::PlayerActionState;
use crate::messages::game_util::ItemStack;
use crate::{
    game::{
        discovery::Discovery, entities::building_state::InventoryState, game_state, permission_helper,
        reducer_helpers::player_action_helpers::post_reducer_update_cargo,
    },
    messages::action_request::*,
    messages::components::*,
    messages::static_data::*,
    unwrap_or_err,
};
use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

#[spacetimedb::reducer]
pub fn paving_place_tile_start(ctx: &ReducerContext, request: PlayerPavingPlaceTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = Some(request.tile_type_id as u64);
    let delay = event_delay(ctx, &request);
    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::PaveTile,
        target,
        None,
        delay,
        self::reduce(ctx, &mut terrain_cache, actor_id, &request, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
pub fn paving_place_tile(ctx: &ReducerContext, request: PlayerPavingPlaceTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::PaveTile.get_layer(ctx),
        self::reduce(ctx, &mut terrain_cache, actor_id, &request, false),
    )
}

fn event_delay(ctx: &ReducerContext, request: &PlayerPavingPlaceTileRequest) -> Duration {
    let recipe = ctx.db.paving_tile_desc().id().find(&request.tile_type_id);
    if recipe.is_none() {
        return Duration::ZERO;
    }

    let recipe = recipe.unwrap();
    return Duration::from_secs_f32(recipe.paving_duration);
}

fn reduce(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    request: &PlayerPavingPlaceTileRequest,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::PaveTile, Some(request.tile_type_id as u64))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::PaveTile, request.timestamp)?;
    }

    // Verify distance to paving target
    let player_mobile = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(&actor_id), "Invalid player");
    let player_coord = player_mobile.coordinates();
    let target_coord = request.coordinates.into();

    if player_coord.distance_to(target_coord) > 3 {
        return Err("Too far".into());
    }

    if !PermissionState::can_interact_with_tile(ctx, actor_id, target_coord, Permission::Build) {
        return Err("You don't have the permission to pave here".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, target_coord, actor_id, ClaimPermission::Build) {
        return Err("You can't pave on this claim.".into());
    }

    if let Some(terrain_source) = terrain_cache.get_terrain_cell(ctx, &player_coord.parent_large_tile()) {
        if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &target_coord.parent_large_tile()) {
            let elevation_diff = i16::abs(terrain_source.elevation - terrain_target.elevation);
            if elevation_diff > 3 {
                return Err("Can't pave down or up a cliff".into());
            }

            if terrain_target.is_submerged() {
                return Err("Can't pave under water".into());
            }

            for biome in ctx.db.biome_desc().disallow_player_build().filter(true) {
                if terrain_target.biome_percentage(Biome::to_enum(biome.biome_type)) > 0f32 {
                    return Err("Can't pave close to a spawn area".into());
                }
            }
        } else {
            return Err("Invalid coordinates".into());
        }
    } else {
        return Err("Invalid coordinates".into());
    }

    for footprint in FootprintTileState::get_at_location(ctx, &target_coord) {
        // Despawn resources under paving
        if let Some(deposit) = ctx.db.resource_state().entity_id().find(footprint.owner_entity_id) {
            let flattenable = ctx.db.resource_desc().id().find(deposit.resource_id).unwrap().flattenable;
            if !flattenable {
                return Err("Can't pave over that resource".into());
            }
            if !dry_run {
                deposit.despawn_self(ctx);
            }
        }
    }

    if !game_state::game_state_filters::is_flat_corner(ctx, terrain_cache, target_coord) {
        return Err("Can only pave flat terrain".into());
    }

    // Delete existing paving
    let existing_paving = PavedTileState::get_at_location(ctx, &target_coord);

    if let Some(ref paving) = existing_paving {
        if paving.tile_type_id == request.tile_type_id {
            return Err("This tile already has that type of pavement".into());
        }
    }

    let mut item_inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    let tile_description = unwrap_or_err!(
        ctx.db.paving_tile_desc().id().find(&request.tile_type_id),
        "Invalid paving tile type"
    );
    let consumed_item_stacks: Vec<ItemStack> = tile_description
        .consumed_item_stacks
        .iter()
        .map(|i| ItemStack::new(ctx, i.item_id, i.item_type, i.quantity))
        .collect();

    let remove_items = consumed_item_stacks.len() > 0;

    if !item_inventory.has(&consumed_item_stacks) {
        return Err("You don't have the required items.".into());
    }

    let mut remove_cargo = false;
    if tile_description.input_cargo_id != 0 {
        let player_cargo_id = InventoryState::get_player_cargo_id(ctx, actor_id);

        if player_cargo_id != tile_description.input_cargo_id {
            return Err("You don't have the required cargo.".into());
        }

        remove_cargo = true;
    }

    if dry_run {
        return Ok(());
    }

    item_inventory.remove(&consumed_item_stacks);

    // Delete existing paving
    if let Some(paving) = existing_paving {
        if claim_helper::get_claim_on_tile(ctx, target_coord).is_some() {
            PavedTileState::refund_paving(ctx, &paving, &mut item_inventory); //Refund materials
        }
        PavedTileState::delete_paving(ctx, &paving.entity_id);
    }

    // TODO: Merge inventories
    if remove_items {
        ctx.db.inventory_state().entity_id().update(item_inventory);
    }

    if remove_cargo && !InventoryState::update_remove_player_cargo(ctx, actor_id) {
        return Err("Failed to update player cargo inventory".into());
    }

    // Create paved tile
    let entity_id = game_state::create_entity(ctx);

    // location
    let offset = target_coord.to_offset_coordinates();
    game_state::insert_location(ctx, entity_id, offset);

    // tile entity
    let paved_tile = PavedTileState {
        entity_id,
        tile_type_id: request.tile_type_id,
        related_entity_id: 0,
    };

    if ctx.db.paved_tile_state().try_insert(paved_tile).is_err() {
        return Err("Failed to insert pavement".into());
    }

    let mut discovery = Discovery::new(actor_id);
    discovery.acquire_paving(ctx, request.tile_type_id);
    discovery.commit(ctx);

    // Grant experience
    if let Some(experience_per_progress) = tile_description.experience_per_progress.get(0) {
        ExperienceState::add_experience(
            ctx,
            actor_id,
            experience_per_progress.skill_id,
            experience_per_progress.quantity as i32,
        );
    }

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
