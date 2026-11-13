use std::collections::HashMap;

use spacetimedb::{log, rand::Rng, ReducerContext, Table};

use crate::{
    agents::resources_regen::*,
    game::{
        coordinates::*,
        handlers::authentication::has_role,
        terrain_chunk::TerrainChunkCache,
        world_gen::resources_log::{resources_log, ResourceClumpInfo},
    },
    messages::{
        authentication::Role,
        components::{ResourceState, TerrainChunkState},
        generic::{resource_count, world_region_state},
        static_data::*,
    },
    unwrap_or_err,
    utils::iter_utils::GroupByAndCount,
};

#[spacetimedb::reducer]
fn admin_resource_force_regen(ctx: &ReducerContext, resource_id: i32, iterations: i32, ignore_target_count: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let resources_log = ctx.db.resources_log().version().find(0);

    if resources_log.is_none() {
        return Ok(());
    }

    let resources_log = resources_log.unwrap();

    let mut terrain_cache = TerrainChunkCache::empty();

    // World scale coordinates
    const CHUNK_WIDTH: u32 = TerrainChunkState::WIDTH;
    const CHUNK_HEIGHT: u32 = TerrainChunkState::HEIGHT;
    let total_count =
        (resources_log.world_width as usize) * CHUNK_WIDTH as usize * (resources_log.world_height as usize) * CHUNK_HEIGHT as usize * 9;
    let region_info = ctx.db.world_region_state().id().find(0).unwrap();
    let (region_offset_x, region_offset_z) = (
        region_info.region_min_chunk_x as i32 * CHUNK_WIDTH as i32 * 3,
        region_info.region_min_chunk_z as i32 * CHUNK_WIDTH as i32 * 3,
    );
    let get_coordinates = |index: usize| {
        let y = (index as u64) / (resources_log.world_width * (CHUNK_WIDTH as u64) * 3);
        let x = (index as u64) - y * resources_log.world_width * (CHUNK_WIDTH as u64) * 3;

        OffsetCoordinatesSmall {
            x: x as i32 + region_offset_x,
            z: y as i32 + region_offset_z,
            dimension: 1,
        }
    };

    let resource_desc: HashMap<i32, ResourceDesc> = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();
    let mut resource_clumps: HashMap<i32, ResourceClumpDesc> = ctx.db.resource_clump_desc().iter().map(|r| (r.id, r)).collect();
    let mut result: Vec<(
        i32,          /* resource id */
        SmallHexTile, /* spawn coord */
        i32,          /* direction */
    )> = Vec::with_capacity(1024);

    // Tally missing resources in the world
    let mut resources_to_delete: Vec<(u64 /* entity id */, i32 /* resource id */)> = Vec::with_capacity(512);
    let mut start_count = 0;
    let mut target = 0;
    if let Some(resource) = resources_log.resources.iter().find(|r| r.resource_id == resource_id) {
        let desc = unwrap_or_err!(resource_desc.get(&resource.resource_id), "Missing ResourceDesc");
        if desc.not_respawning {
            return Err(format!("{} does not respawn.", desc.name));
        }
        let resource_count = unwrap_or_err!(
            ctx.db.resource_count().resource_id().find(resource.resource_id),
            "Missing resource count"
        )
        .num_in_world;
        if resource_count >= resource.world_target && !ignore_target_count {
            return Err(format!("{} already at target count.", desc.name));
        }

        let mut missing_count = resource.world_target - resource_count;
        start_count = resource_count;
        target = resource.world_target;
        log::info!(
            "Resource {} (id: {}) - [{}/{}] - {:.2}%",
            desc.name,
            resource.resource_id,
            resource_count,
            resource.world_target,
            (resource_count as f32 / resource.world_target as f32) * 100f32
        );

        let mut req_resources_in_clumps: HashMap<
            i32, /* clump id */
            HashMap<i32 /* resource id */, usize /* resource count in clump */>,
        > = HashMap::with_capacity(32);
        let mut clump_info_lookup: HashMap<i32 /* clump id */, &ResourceClumpInfo> = HashMap::with_capacity(32);
        for clump_info in &resources_log.resource_clumps {
            let clump_desc = resource_clumps.get(&clump_info.clump_id).unwrap();
            if clump_desc.resource_id.iter().any(|r| *r != resource_id) {
                // clump contains a resource that's not missing or doesn't respawn, so it cannot spawn
                continue;
            }
            let req_resources = clump_desc.resource_id.iter().group_by_and_count(|r| **r);
            req_resources_in_clumps.insert(clump_info.clump_id, req_resources);
            clump_info_lookup.insert(clump_info.clump_id, clump_info);
        }
        let clump_desc_extended: HashMap<i32, ResourceClumpDescExtended> = resource_clumps
            .drain()
            .map(|a| (a.0, ResourceClumpDescExtended::new(a.1, &resource_desc)))
            .collect();

        // Precompute the set of tiles occupied by claims, so that resources can't spawn on them.
        // TODO: Determine whether it would be faster to use a `LazyMemoOccupiedTiles` here.
        let mut occupied_tiles_hashes = PrecomputeOccupiedTiles::new(ctx);

        if !req_resources_in_clumps.is_empty() {
            // attempt to place each clump into the map in worldgen order until no more resource exists.
            'outer_loop: for req in &req_resources_in_clumps {
                let req_resources = req.1;

                let clump_info = *clump_info_lookup.get(req.0).unwrap();
                let clump_desc = clump_desc_extended.get(&clump_info.clump_id).unwrap();

                for _ in 0..iterations {
                    let tile_index = ctx.rng().gen_range(0..total_count - 1);
                    let offset_coordinates = get_coordinates(tile_index);
                    let hex_coordinates = SmallHexTile::from(offset_coordinates);
                    if try_spawn_resource(
                        ctx,
                        &mut terrain_cache,
                        clump_info,
                        clump_desc,
                        hex_coordinates,
                        &mut occupied_tiles_hashes,
                        //false,
                        &resource_desc,
                        &mut resources_to_delete,
                        &mut result,
                    ) {
                        //log::info!("---- Regen success");
                        for count in req_resources.values() {
                            if missing_count <= *count as i32 && !ignore_target_count {
                                break 'outer_loop;
                            }
                            missing_count -= *count as i32;
                        }
                    }
                }
            }
        } else {
            return Err("No suitable resource clump".into());
        }
    }

    if !result.is_empty() {
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

        for &(resource_id, coordinates, direction) in &result {
            let resource_desc = resource_desc.get(&resource_id).unwrap();
            // TODO: mirror `ResourceCount` counters into WASM memory,
            //       update the in-memory version for each inserted resource,
            //       then do one `ResourceCount::update_by_resource_id` for each type
            //       after all insertions,
            //       rather than having `resource_spawn::spawn` update the counter each time.
            ResourceState::spawn(
                ctx,
                None,
                resource_id,
                coordinates,
                direction,
                resource_desc.max_health,
                false,
                false,
            )
            .unwrap();
        }

        let mut spawn_hashmap: HashMap<i32, (String, i32)> = HashMap::new();
        let res_count = result.iter().group_by_and_count(|a| a.0);
        for (resource_id, count) in res_count {
            let resource_desc = resource_desc.get(&resource_id).unwrap();
            spawn_hashmap.insert(resource_desc.id, (resource_desc.name.clone(), count as i32));
        }

        for (_, (resource_name, count)) in spawn_hashmap {
            let previous_count = start_count;
            let target = target;
            let previous_pct = (previous_count as f32 / target as f32) * 100f32;
            let new_pct = ((previous_count + count) as f32 / target as f32) * 100f32;

            log::info!(
                "Respawned resource {} => {} ({:.2}% => {:.2}%)",
                resource_name,
                count,
                previous_pct,
                new_pct
            );
        }
    }

    Ok(())
}
