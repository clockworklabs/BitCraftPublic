use crate::game::game_state::game_state_filters;
use crate::game::reducer_helpers;
use crate::{
    game::{coordinates::*, game_state, terrain_chunk::TerrainChunkCache},
    messages::{action_request::PlayerDeployableMoveRequest, components::*, static_data::*},
    unwrap_or_err,
};
use spacetimedb::ReducerContext;

// All coordinates (player position, deployable position, source coordinates, target coordinates) should be within this distance of each other for a follow to occur
// This is a safety measure to prevent both client bugs and client cheating
// Increase if too strict and preventing legitimate follows
const MAX_PROXIMITY_DISTANCE: f32 = 100.0;

#[spacetimedb::reducer]
pub fn deployable_follow(ctx: &ReducerContext, request: PlayerDeployableMoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    let deployable_entity_id = request.deployable_entity_id;

    if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&actor_id) {
        if mounting.deployable_entity_id == deployable_entity_id {
            return Err("~Mounted deployable cannot follow a player".into());
        }
    }

    let origin = unwrap_or_err!(request.origin, "~Expected origin in move request");
    let dest = unwrap_or_err!(request.destination, "~Expected destination in move request");
    let source_coordinates: FloatHexTile = origin.into();
    let target_coordinates: FloatHexTile = dest.into();

    // Both these functions panic if the entity is not found in mobile entity
    let deployable_location = game_state_filters::coordinates_float(ctx, deployable_entity_id);
    let player_location = game_state_filters::coordinates_float(ctx, actor_id);

    // We now have 4 points to check:
    let points_to_check = [
        deployable_location, // Deployable location (from server)
        player_location,     // Player location (from server)
        source_coordinates,  // Source coordinates (from client)
        target_coordinates,  // Target coordinates (from client)
    ];

    //ALL of these points should be in the same dimension and within MAX_PROXIMITY_DISTANCE of each other
    //Otherwise, client is bugged or cheating
    all_close_or_err(&points_to_check, MAX_PROXIMITY_DISTANCE, "~Invalid coordinates")?;

    // Further validation checks
    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_start = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &source_coordinates.parent_large_tile()),
        "~You can't go here!",
    );
    let terrain_target = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &target_coordinates.parent_large_tile()),
        "~You can't go here!",
    );
    let deployable_location_terrain = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &deployable_location.parent_large_tile()),
        "~You can't go here!",
    );

    let deployable = unwrap_or_err!(
        ctx.db.deployable_state().entity_id().find(&deployable_entity_id),
        "~Deployable not found!"
    );

    if deployable.owner_id != actor_id {
        return Err("~You don't own that deployable".into());
    }

    let desc = unwrap_or_err!(
        ctx.db.deployable_desc().id().find(&deployable.deployable_description_id),
        "~Invalid deployable type"
    );

    let pathfinding = unwrap_or_err!(
        ctx.db.pathfinding_desc().id().find(&desc.pathfinding_id),
        "~Invalid deployable pathfinding info"
    );

    // Only checking water depth at the client-provided coordinates is unsafe, we cannot trust the client
    // At the very least, we should also check the water depth at the deployable's ACTUAL location
    let start_water_depth = terrain_start.water_depth() as i32;
    let target_water_depth = terrain_target.water_depth() as i32;
    let deployable_location_water_depth = deployable_location_terrain.water_depth() as i32;

    let start_ok = start_water_depth >= pathfinding.min_water_depth && start_water_depth <= pathfinding.max_water_depth;
    let target_ok = target_water_depth >= pathfinding.min_water_depth && target_water_depth <= pathfinding.max_water_depth;
    let deployable_location_ok = deployable_location_water_depth >= pathfinding.min_water_depth && deployable_location_water_depth <= pathfinding.max_water_depth;

    // Previously, this checked if start AND target were invalid
    // That would allow a deployable to move from land->water or water->land (in both cases, one endpoint is valid)
    // We should instead ensure start, target, and deployable location are all valid
    if !start_ok || !target_ok || !deployable_location_ok {
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
    reducer_helpers::deployable_helpers::move_deployable(ctx, deployable_entity_id, origin, dest, request.timestamp, request.duration)
}

pub fn all_close_or_err(coords: &[FloatHexTile], max_dist: f32, msg: &str) -> Result<(), String> {
    if coords.len() <= 1 {
        return Ok(());
    }

    // Since currently distance_to() does not check dimensions, we need to check them manually
    let dim = coords[0].dimension;
    if coords.iter().any(|c| c.dimension != dim) {
        return Err(msg.into());
    }

    for i in 0..coords.len() {
        for j in (i + 1)..coords.len() {
            if coords[i].distance_to(coords[j]) > max_dist {
                return Err(msg.into());
            }
        }
    }

    Ok(())
}
