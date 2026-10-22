use bitcraft_macro::feature_gate;
use std::time::Duration;

use crate::game::coordinates::*;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::PlayerActionState;
use crate::messages::events::*;
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

#[spacetimedb::reducer]
#[feature_gate]
pub fn pillar_shaping_place_pillar_start(ctx: &ReducerContext, request: PlayerPillarShapingPlaceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = Some(request.pillar_type_id as u64);
    let delay = event_delay(ctx, &request);
    let mut terrain_cache = TerrainChunkCache::empty();
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::PlacePillarShaping,
        target,
        None,
        delay,
        self::reduce(ctx, &mut terrain_cache, actor_id, &request, true),
        request.timestamp,
    );
    if result.is_ok() {
        ctx.db
            .pillar_shaping_place_pillar_start_event()
            .insert(PillarShapingPlacePillarStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn pillar_shaping_place_pillar(ctx: &ReducerContext, request: PlayerPillarShapingPlaceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::PlacePillarShaping.get_layer(ctx),
        self::reduce(ctx, &mut terrain_cache, actor_id, &request, false),
    )
}

fn event_delay(ctx: &ReducerContext, request: &PlayerPillarShapingPlaceRequest) -> Duration {
    let recipe = ctx.db.pillar_shaping_desc().id().find(&request.pillar_type_id);
    if recipe.is_none() {
        return Duration::ZERO;
    }

    let recipe = recipe.unwrap();
    return Duration::from_secs_f32(recipe.duration);
}

fn reduce(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    request: &PlayerPillarShapingPlaceRequest,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(
            ctx,
            actor_id,
            PlayerActionType::PlacePillarShaping,
            Some(request.pillar_type_id as u64),
        )?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::PlacePillarShaping, request.timestamp)?;
    }

    // Verify distance to paving target
    let coordinates: LargeHexTile = request.coordinates.into();
    let player_mobile = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(&actor_id), "Invalid player");
    let player_coord = player_mobile.coordinates();
    let target_coord = coordinates.center_small_tile();

    if player_coord.distance_to(target_coord) > 3 {
        return Err("Too far".into());
    }

    if !PermissionState::can_interact_with_tile(ctx, actor_id, target_coord, Permission::Build) {
        return Err("You don't have the permission to shape pillars here".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, target_coord, actor_id, ClaimPermission::Build) {
        return Err("You can't add pillar decoration on this claim.".into());
    }

    if game_state::game_state_filters::floating_platform_at_coordinates(ctx, target_coord).is_some() {
        return Err("Can't add pillar decoration on a floating platform".into());
    }

    if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &coordinates) {
        if terrain_target.is_submerged() {
            return Err("Can't add pillar decoration under water".into());
        }

        for biome in ctx.db.biome_desc().disallow_player_build().filter(true) {
            if terrain_target.biome_percentage(Biome::to_enum(biome.biome_type)) > 0f32 {
                return Err("Can't add pillar decoration in this biome".into());
            }
        }
    } else {
        return Err("Invalid coordinates".into());
    }

    // Delete existing pillar
    if let Some(existing_pillar_shaping) = PillarShapingState::get_at_location(ctx, &coordinates) {
        if existing_pillar_shaping.pillar_type_id == request.pillar_type_id {
            return Err("This pillar already has that type of decoration".into());
        }
        if !dry_run {
            PillarShapingState::delete_pillar_shaping(ctx, existing_pillar_shaping.entity_id);
        }
    }

    let mut item_inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    let pillar_shaping_recipe = unwrap_or_err!(
        ctx.db.pillar_shaping_desc().id().find(&request.pillar_type_id),
        "Invalid paving tile type"
    );
    let consumed_item_stacks: Vec<ItemStack> = pillar_shaping_recipe
        .consumed_item_stacks
        .iter()
        .map(|i| ItemStack::new(ctx, i.item_id, i.item_type, i.quantity))
        .collect();

    let remove_items = consumed_item_stacks.len() > 0;

    if !item_inventory.has(&consumed_item_stacks) {
        return Err("You don't have the required items.".into());
    }

    item_inventory.remove(&consumed_item_stacks);

    let mut remove_cargo = false;
    if pillar_shaping_recipe.input_cargo_id != 0 {
        let player_cargo_id = InventoryState::get_player_cargo_id(ctx, actor_id);

        if player_cargo_id != pillar_shaping_recipe.input_cargo_id {
            return Err("You don't have the required cargo.".into());
        }

        remove_cargo = true;
    }

    if dry_run {
        return Ok(());
    }

    // TODO: Merge inventories
    if remove_items {
        ctx.db.inventory_state().entity_id().update(item_inventory);
    }

    if remove_cargo && !InventoryState::update_remove_player_cargo(ctx, actor_id) {
        return Err("Failed to update player cargo inventory".into());
    }

    // Create pillar shaping tile
    let entity_id = game_state::create_entity(ctx);

    // location
    let offset = target_coord.to_offset_coordinates();
    game_state::insert_location(ctx, entity_id, offset);

    // pillar entity
    let pillar = PillarShapingState {
        entity_id,
        pillar_type_id: request.pillar_type_id,
    };

    if ctx.db.pillar_shaping_state().try_insert(pillar).is_err() {
        return Err("Failed to insert pillar shaping".into());
    }

    let mut discovery = Discovery::new(actor_id);
    discovery.acquire_pillar_shaping(ctx, request.pillar_type_id);
    discovery.commit(ctx);

    // Grant experience
    if let Some(experience_per_progress) = pillar_shaping_recipe.experience_per_progress.get(0) {
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
