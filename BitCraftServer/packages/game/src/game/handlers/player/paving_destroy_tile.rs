use bitcraft_macro::feature_gate;
use std::time::Duration;

use crate::game::game_state;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::PlayerActionState;
use crate::messages::events::*;
use crate::table_caches::claim_tile_state_cache::ClaimTileStateCache;
use crate::table_caches::location_state_cache::LocationStateCache;
use crate::{game::permission_helper, messages::action_request::PlayerPavingDestroyTileRequest, messages::components::*, unwrap_or_err};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn paving_destroy_tile_start(ctx: &ReducerContext, request: PlayerPavingDestroyTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = Some(request.coordinates.hashcode_long() as u64);
    let delay = event_delay(actor_id, &request);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::DestroyPaving,
        target,
        None,
        delay,
        self::reduce(ctx, actor_id, &request, true),
        request.timestamp,
    );
    if result.is_ok() {
        ctx.db
            .paving_destroy_tile_start_event()
            .insert(PavingDestroyTileStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn paving_destroy_tile(ctx: &ReducerContext, request: PlayerPavingDestroyTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::DestroyPaving.get_layer(ctx),
        self::reduce(ctx, actor_id, &request, false),
    )
}

fn event_delay(_actor_id: u64, _request: &PlayerPavingDestroyTileRequest) -> Duration {
    // DAB Notes: put that in parameters.csv?
    Duration::from_secs(3)
}

fn reduce(ctx: &ReducerContext, actor_id: u64, request: &PlayerPavingDestroyTileRequest, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(
            ctx,
            actor_id,
            PlayerActionType::DestroyPaving,
            Some(request.coordinates.hashcode_long() as u64),
        )?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::DestroyPaving, request.timestamp)?;
    }

    // Verify distance to paving target
    let player_mobile = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Invalid player");
    let player_coord = player_mobile.coordinates();
    let target_coord = request.coordinates.into();

    if player_coord.distance_to(target_coord) > 3 {
        return Err("Too far".into());
    }

    if !PermissionState::can_interact_with_tile(ctx, actor_id, target_coord, Permission::Build) {
        return Err("You don't have the permission to remove the paving".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, target_coord, actor_id, ClaimPermission::Build) {
        return Err("You can't remove paving on this claim.".into());
    }

    let mut terrain_cache = TerrainChunkCache::empty();
    if let Some(terrain_source) = terrain_cache.get_terrain_cell(ctx, &player_coord.parent_large_tile()) {
        if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &target_coord.parent_large_tile()) {
            let elevation_diff = i16::abs(terrain_source.elevation - terrain_target.elevation);
            if elevation_diff > 3 {
                return Err("Can't destroy paving up or down a cliff".into());
            }

            if terrain_target.is_submerged() {
                // In case water level changes?
                return Err("Can't destroy paving under water".into());
            }
        } else {
            return Err("Invalid coordinates".into());
        }
    } else {
        return Err("Invalid coordinates".into());
    }

    let paving = unwrap_or_err!(PavedTileState::get_at_location(ctx, &target_coord), "Tile is not paved");
    if !dry_run {
        let paving_entity_id = paving.entity_id;
        PavedTileState::delete_paving(ctx, &paving_entity_id);

        let mut location_cache = LocationStateCache::new();
        let mut claim_cache = ClaimTileStateCache::new(&mut location_cache);
        if claim_cache.get_claim_on_tile(ctx, target_coord).is_none() {
            return Ok(()); //Don't refund materials on unclaimed tiles
        }

        //Refund materials
        let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
        PavedTileState::refund_paving(ctx, &paving, &mut inventory);
        inventory.update(ctx);
    }

    Ok(())
}
