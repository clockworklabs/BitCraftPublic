use crate::agents::resources_regen::{try_spawn_resource_options, PrecomputeOccupiedTiles, ResourceClumpDescExtended};
use crate::game::coordinates::SmallHexTile;
use crate::game::dimensions::OVERWORLD;
use crate::game::game_state;
use crate::game::handlers::authentication::has_role;
use crate::game::reducer_helpers::dimension_helpers::{get_dimension_bounds, is_within_dimension_bounds_small};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::world_gen::resources_log::{resources_log, ResourceInfo};
use crate::messages::authentication::Role;
use crate::messages::components::ResourceState;
use crate::messages::generic::resource_count;
use crate::messages::static_data::{resource_clump_desc, resource_desc, ResourceClumpDesc, ResourceDesc};
use crate::messages::util::OffsetCoordinatesSmallMessage;
use crate::messages::world::{
    world_entity_placement_results, WorldEntityPlacement, WorldEntityPlacementResults, WorldPlaceResourceRequest, WorldPlacementType,
};
use crate::utils::iter_utils::GroupByAndCount;
use spacetimedb::{log, ReducerContext, Table};
use std::collections::HashMap;

#[spacetimedb::reducer]
pub fn world_place_resource(ctx: &ReducerContext, request: WorldPlaceResourceRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    //find resource log
    let resources_log = ctx.db.resources_log().version().find(0);
    if resources_log.is_none() {
        return Err("No resource log found".into());
    }

    //always clear results table first
    for row in ctx.db.world_entity_placement_results().iter() {
        ctx.db.world_entity_placement_results().delete(row);
    }

    let resource_desc: HashMap<i32, ResourceDesc> = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();
    let mut resource_clumps: HashMap<i32, ResourceClumpDesc> = ctx.db.resource_clump_desc().iter().map(|r| (r.id, r)).collect();
    let clump_desc_extended: HashMap<i32, ResourceClumpDescExtended> = resource_clumps
        .drain()
        .map(|a| (a.0, ResourceClumpDescExtended::new(a.1, &resource_desc)))
        .collect();

    let mut terrain_cache = TerrainChunkCache::empty();
    let mut occupied_tiles_hashes = PrecomputeOccupiedTiles::new(ctx);

    let len = request.resources.len();

    let mut result: Vec<(
        i32,          /* resource id */
        SmallHexTile, /* spawn coord */
        i32,          /* direction */
    )> = Vec::with_capacity(len);
    let mut resources_to_delete: Vec<(u64 /* entity id */, i32 /* resource id */)> = Vec::with_capacity(len);

    let clump_info = request.resource_clump_info.clone();

    //todo assume dimension is OVERWORLD for now, but possibly get dimension from first placements
    let bounds = get_dimension_bounds(ctx, &OVERWORLD);

    //try to spawn resources
    for resource_placement in request.resources.iter() {
        //bounds check
        if !is_within_dimension_bounds_small(&resource_placement.coordinates, bounds) {
            // log::info!(
            //     "position ({}, {}) is outside world!",
            //     resource_placement.coordinates.x,
            //     resource_placement.coordinates.z
            // );
            continue;
        }

        let hex_coordinates = SmallHexTile::from(resource_placement.coordinates);
        let clump_desc = clump_desc_extended.get(&clump_info.clump_id).unwrap();

        // log::info!(
        //     "Attempt place clump {} at {}, {}",
        //     clump_info.clump_id,
        //     resource_placement.coordinates.x,
        //     resource_placement.coordinates.z
        // );

        if try_spawn_resource_options(
            ctx,
            &mut terrain_cache,
            &clump_info,
            clump_desc,
            hex_coordinates,
            &mut occupied_tiles_hashes,
            &resource_desc,
            &mut resources_to_delete,
            &mut result,
            request.ignore_biome,
            true,
        ) {
            // log::info!("[SUCCESS] clump {} at {}, {}", clump_info.clump_id, resource_placement.coordinates.x, resource_placement.coordinates.z);
        }
    }

    let mut placements: Vec<WorldEntityPlacement> = Vec::new();

    //from resources_regen
    if !result.is_empty() {
        let mut spawned_counts: HashMap<i32, i32> = HashMap::new();

        if !request.dry_run {
            let resources_to_delete_count = resources_to_delete.iter().group_by_and_count(|r| r.1);
            for (entity_id, resource_id) in resources_to_delete {
                ResourceState::despawn(ctx, entity_id, resource_id);
            }

            for (resource_id, delete_count) in resources_to_delete_count {
                let mut count = ctx
                    .db
                    .resource_count()
                    .resource_id()
                    .find(resource_id)
                    .unwrap_or_else(|| panic!("No ResourceCount for resource_id: {}", resource_id));
                count.num_in_world -= delete_count as i32;
                ctx.db.resource_count().resource_id().update(count);
            }
        }

        for &(resource_id, coordinates, direction) in &result {
            let resource_desc = resource_desc.get(&resource_id).unwrap();
            // TODO: mirror `ResourceCount` counters into WASM memory,
            //       update the in-memory version for each inserted resource,
            //       then do one `ResourceCount::update_by_resource_id` for each type
            //       after all insertions,
            //       rather than having `resource_spawn::spawn` update the counter each time.

            //only spawn if not dry run
            let mut resource_entity_id: u64 = 0;
            if !request.dry_run {
                resource_entity_id = ResourceState::spawn(
                    ctx,
                    None,
                    resource_id,
                    coordinates,
                    direction,
                    resource_desc.max_health,
                    false,
                    false,
                )?;
            }

            log::info!("Spawning resource {} at {}", resource_id, coordinates);

            //always log entry
            *spawned_counts.entry(resource_id).or_insert(0) += 1;
            placements.push(WorldEntityPlacement {
                coordinates: OffsetCoordinatesSmallMessage::from(coordinates),
                prototype_id: resource_id,
                entity_id: resource_entity_id,
                placement_type: WorldPlacementType::Resource,
            });
        }

        //only add to resource log if not dry run and we add to resource log
        if !request.dry_run && request.add_to_resources_log {
            //find/update clump infos
            let mut resources_log = resources_log.unwrap();

            let clump_id = request.resource_clump_info.clump_id;

            //insert clump info if not found
            if !resources_log.resource_clumps.iter().any(|c| c.clump_id == clump_id) {
                resources_log.resource_clumps.push(request.resource_clump_info.clone());
            }

            //insert clump info if it doesn't exist
            if !resources_log.resource_clumps.iter().any(|c| c.clump_id == clump_id) {
                resources_log.resource_clumps.push(request.resource_clump_info.clone());
            }

            //update resource log spawn count target
            for (rid, cnt) in spawned_counts {
                if let Some(info) = resources_log.resources.iter_mut().find(|r| r.resource_id == rid) {
                    // update the existing entry
                    info.world_target += cnt;
                } else {
                    // insert new
                    resources_log.resources.push(ResourceInfo {
                        resource_id: rid,
                        world_target: cnt,
                    });
                }
            }

            //update resource log
            ctx.db.resources_log().version().update(resources_log);
        }
    }

    //log placement results
    if request.log_results {
        let placement_results = WorldEntityPlacementResults {
            entity_id: game_state::create_entity(ctx),
            timestamp: game_state::unix(ctx.timestamp),
            add_to_resources_log: request.add_to_resources_log,
            dry_run: request.dry_run,
            placements,
        };

        ctx.db.world_entity_placement_results().try_insert(placement_results)?;
    }

    Ok(())
}
