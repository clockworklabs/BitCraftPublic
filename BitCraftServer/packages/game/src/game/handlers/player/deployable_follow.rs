use bitcraft_macro::feature_gate;
use crate::game::game_state::game_state_filters;
use crate::game::reducer_helpers;
use crate::{
    game::{coordinates::*, game_state, terrain_chunk::TerrainChunkCache},
    messages::{action_request::PlayerDeployableMoveRequest, components::*, static_data::*},
    unwrap_or_err,
};
use crate::messages::events::*;
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_follow(ctx: &ReducerContext, request: PlayerDeployableMoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let deployable_entity_id = request.deployable_entity_id;

    if let Some(mounting) = ctx.db.mounting_state().entity_id().find(actor_id) {
        if mounting.deployable_entity_id == deployable_entity_id {
            return Err("~Mounted deployable cannot follow a player".into());
        }
    }

    let _ = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(deployable_entity_id),
        "~Deployable is no longer available"
    );

    let origin = unwrap_or_err!(request.origin, "~Expected origin in move request");
    let dest = unwrap_or_err!(request.destination, "~Expected destination in move request");
    let source_coordinates: FloatHexTile = origin.into();
    let target_coordinates: FloatHexTile = dest.into();

    let player_location = game_state_filters::coordinates_float(ctx, actor_id);
    if target_coordinates.dimension != player_location.dimension || target_coordinates.distance_to(player_location) > 100.0 {
        return Err("~Deployable destination is too far from its owner position".into());
    }

    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_start = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &source_coordinates.parent_large_tile()),
        "~You can't go here!",
    );
    let terrain_target = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &target_coordinates.parent_large_tile()),
        "~You can't go here!",
    );

    // Validate Boat Move. Note that this for the short term, ideally at some point
    // we will re-enable the above _validate_move that has more comprehensive checks
    let deployable = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id),
        "~Deployable not found!"
    );

    if deployable.owner_id != actor_id {
        return Err("~You don't own that deployable".into());
    }

    let desc = unwrap_or_err!(
        ctx.db.deployable_desc().id().find(deployable.deployable_description_id),
        "~Invalid deployable type"
    );

    let pathfinding = unwrap_or_err!(
        ctx.db.pathfinding_desc().id().find(desc.pathfinding_id),
        "~Invalid deployable pathfinding info"
    );

    let start_water_depth = terrain_start.water_depth() as i32;
    let target_water_depth = terrain_target.water_depth() as i32;

    if (start_water_depth < pathfinding.min_water_depth || start_water_depth > pathfinding.max_water_depth)
        && (target_water_depth < pathfinding.min_water_depth || target_water_depth > pathfinding.max_water_depth)
    {
        return Err("~You can't go here!".into());
    }

    // update deployable map location
    let mut deployable_collectible = unwrap_or_err!(
        ctx.db
            .deployable_collectible_state()
            .deployable_entity_id()
            .find(deployable_entity_id),
        "No deployable collectible state"
    );
    if !deployable_collectible.auto_follow {
        return Err("This deployable is not set to auto-follow".into());
    }

    let coord = source_coordinates.parent_small_tile();
    deployable_collectible.location = Some(coord.into());
    ctx.db
        .deployable_collectible_state()
        .deployable_entity_id()
        .update(deployable_collectible);

    // update deployable location
    reducer_helpers::deployable_helpers::move_deployable(ctx, deployable_entity_id, origin, dest, request.timestamp, request.duration)?;
    ctx.db.deployable_move_event().insert(DeployableMoveEvent {
        actor_id,
        request,
        is_follow: true,
    });
    Ok(())
}
