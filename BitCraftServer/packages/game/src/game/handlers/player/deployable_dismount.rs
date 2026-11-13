use bitcraft_macro::feature_gate;
use crate::game::game_state::game_state_filters;
use crate::game::reducer_helpers::deployable_helpers::{
    dismount_deployable_and_explore, dismount_deployable_and_explore_and_set_deployable_position,
};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::messages::action_request::PlayerDeployableDismountRequest;
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::*;
use crate::{parameters_desc, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::table(accessor = deployable_dismount_timer, scheduled(deployable_dismount_scheduled, at = scheduled_at))]
pub struct DeployableDismountTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub deployable_entity_id: u64,
    pub player_entity_id: u64, // for server request when storing a populated deployable
    pub coordinates: Option<OffsetCoordinatesFloat>,
    pub skip_deployable_icon: bool,
}

#[spacetimedb::reducer]
#[feature_gate]
fn deployable_dismount_scheduled(ctx: &ReducerContext, timer: DeployableDismountTimer) -> Result<(), String> {
    deployable_dismount_server(
        ctx,
        PlayerDeployableDismountRequest {
            deployable_entity_id: timer.deployable_entity_id,
            player_entity_id: timer.player_entity_id,
            coordinates: timer.coordinates,
            skip_deployable_icon: timer.skip_deployable_icon,
            deployable_coordinates: None,
        },
    )
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_dismount_server(ctx: &ReducerContext, request: PlayerDeployableDismountRequest) -> Result<(), String> {
    // This request can only come from the server (as a result of a deployable being stored)
    ServerIdentity::validate_server_only(ctx)?;
    let actor_id = request.player_entity_id;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    // Deployable might be deleted at this point if everyone was expelled from it. If this happens, we have a target location so we will use that.
    let deployable_coord = if let Some(deployable) = ctx.db.mobile_entity_state().entity_id().find(request.deployable_entity_id) {
        deployable.coordinates_float()
    } else {
        request.coordinates.unwrap().into()
    };

    let target_coordinates: FloatHexTile = unwrap_or_err!(request.coordinates, "No location to disembark to").into();
    self::test_coordinates(ctx, target_coordinates, deployable_coord)?;

    PlayerState::collect_stats(ctx, actor_id);
    if let Some(deployable_coordinates) = request.deployable_coordinates {
        self::test_coordinates(ctx, deployable_coordinates.into(), deployable_coord)?;
        dismount_deployable_and_explore_and_set_deployable_position(
            ctx,
            actor_id,
            target_coordinates,
            deployable_coordinates.into(),
            request.skip_deployable_icon,
        )
    } else {
        dismount_deployable_and_explore(ctx, actor_id, target_coordinates, request.skip_deployable_icon)
    }
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_dismount(
    ctx: &ReducerContext,
    coordinates: OffsetCoordinatesFloat,
    deployable_coordinates: OffsetCoordinatesFloat,
) -> Result<(), String> {
    // This request can only come from a player (as a result of a player direct action)
    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mounting_state = unwrap_or_err!(ctx.db.mounting_state().entity_id().find(actor_id), "You're not in a deployable");
    let deployable_coord = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(mounting_state.deployable_entity_id),
        "Deployable doesn't exist"
    )
    .coordinates_float();

    let target_coordinates: FloatHexTile = coordinates.into();
    self::test_coordinates(ctx, target_coordinates, deployable_coord)?;

    PlayerState::collect_stats(ctx, actor_id);
    self::test_coordinates(ctx, deployable_coordinates.into(), deployable_coord)?;
    dismount_deployable_and_explore_and_set_deployable_position(ctx, actor_id, target_coordinates, deployable_coordinates.into(), false)
}

fn test_coordinates(ctx: &ReducerContext, dismount_coord: FloatHexTile, deployable_coord: FloatHexTile) -> Result<(), String> {
    let (dismount_coord_small, dismount_coord_large) = dismount_coord.parent_small_and_large_tile();
    let mut terrain_cache = TerrainChunkCache::empty();
    let dismount_terrain_cell = match terrain_cache.get_terrain_cell(ctx, &dismount_coord_large) {
        Some(tc) => tc,
        None => return Err("Can't find dismount cell".into()),
    };

    if dismount_coord.distance_to(deployable_coord) > 5.0 {
        spacetimedb::log::warn!(
            "Can't disembark this far! {} to {} = {}",
            dismount_coord,
            deployable_coord,
            dismount_coord.distance_to(deployable_coord)
        );
        return Err("Can't disembark this far!".into());
    }

    if game_state_filters::has_hitbox_footprint(ctx, dismount_coord_small) {
        return Err("Can't disembark there!".into());
    }

    let deployable_terrain_cell = match terrain_cache.get_terrain_cell(ctx, &deployable_coord.parent_large_tile()) {
        Some(tc) => tc,
        None => return Err("Can't find deployable cell".into()),
    };

    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    if (dismount_terrain_cell.player_surface_elevation() - deployable_terrain_cell.player_surface_elevation()).abs()
        > params.deployable_disembark_max_elevation as i16
    {
        return Err("Can't disembark over a cliff!".into());
    }

    Ok(())
}
