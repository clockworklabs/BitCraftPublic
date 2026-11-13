use bitcraft_macro::feature_gate;
use std::time::Duration;

use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::messages::components::PlayerActionState;
use crate::messages::events::*;
use crate::{game::permission_helper, messages::action_request::PlayerPillarShapingDestroyRequest, messages::components::*, unwrap_or_err};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn pillar_shaping_destroy_start(ctx: &ReducerContext, request: PlayerPillarShapingDestroyRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = Some(request.coordinates.hashcode_long() as u64);
    let delay = event_delay(actor_id, &request);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::DestroyPillarShaping,
        target,
        None,
        delay,
        self::reduce(ctx, actor_id, &request, true),
        request.timestamp,
    );
    if result.is_ok() {
        ctx.db
            .pillar_shaping_destroy_start_event()
            .insert(PillarShapingDestroyStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn pillar_shaping_destroy(ctx: &ReducerContext, request: PlayerPillarShapingDestroyRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::DestroyPillarShaping.get_layer(ctx),
        self::reduce(ctx, actor_id, &request, false),
    )
}

fn event_delay(_actor_id: u64, _request: &PlayerPillarShapingDestroyRequest) -> Duration {
    // DAB Notes: put that in parameters.csv?
    Duration::from_secs(3)
}

fn reduce(ctx: &ReducerContext, actor_id: u64, request: &PlayerPillarShapingDestroyRequest, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(
            ctx,
            actor_id,
            PlayerActionType::DestroyPillarShaping,
            Some(request.coordinates.hashcode_long() as u64),
        )?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::DestroyPillarShaping, request.timestamp)?;
    }

    // Verify distance to paving target
    let coordinates: LargeHexTile = request.coordinates.into();
    let player_mobile = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Invalid player");
    let player_coord = player_mobile.coordinates();
    let target_coord = coordinates.center_small_tile();

    if player_coord.distance_to(target_coord) > 3 {
        return Err("Too far".into());
    }

    if !PermissionState::can_interact_with_tile(ctx, actor_id, target_coord, Permission::Build) {
        return Err("You don't have the permission to shape pillars here".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, target_coord, actor_id, ClaimPermission::Build) {
        return Err("You can't shape pillars on this claim.".into());
    }

    let mut terrain_cache = TerrainChunkCache::empty();
    if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &coordinates) {
        if terrain_target.is_submerged() {
            // In case water level changes?
            return Err("Can't destroy pillar shaping under water".into());
        }
    } else {
        return Err("Invalid coordinates".into());
    }

    let pillar = unwrap_or_err!(
        PillarShapingState::get_at_location(ctx, &coordinates),
        "No pillar decoration on this tile"
    );
    if !dry_run {
        PillarShapingState::delete_pillar_shaping(ctx, pillar.entity_id);
    }

    Ok(())
}
