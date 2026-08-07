use std::collections::{HashMap, HashSet};
use std::time::Duration;

use spacetimedb::rand::Rng;
use spacetimedb::{log, ReducerContext, Table};

use crate::game::coordinates::*;
use crate::game::game_state::game_state_filters;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::unity_helpers::vector2::Vector2;
use crate::game::world_gen::resources_log::{resources_log, ResourceClumpInfo};
use crate::game::world_gen::{noise_helper, world_definition};
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::{claim_tile_state, location_state, FootprintTileState, ResourceState};
use crate::messages::generic::{resource_count, world_region_state, ResourceCount};
use crate::messages::static_data::*;
use crate::messages::util::SmallHexTileMessage;
use crate::utils::iter_utils::GroupByAndCount;
use crate::{agents, building_state, footprint_tile_state, paved_tile_state, resource_state, LocationState};

const CHUNK_WIDTH: usize = world_definition::TERRAIN_CHUNK_WIDTH as usize;
const CHUNK_HEIGHT: usize = world_definition::TERRAIN_CHUNK_HEIGHT as usize;

#[spacetimedb::table(name = resources_regen_loop_timer, scheduled(resources_regen, at = scheduled_at)
)]
pub struct ResourcesRegenLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(&0).unwrap();
    let mut count = 0;
    for mut timer in ctx.db.resources_regen_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(params.resources_regen_tick_millis as u64).into();
        ctx.db.resources_regen_loop_timer().scheduled_id().update(timer);
        log::info!("resources regen agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one Resource Regen agent running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(&0).unwrap();
    ctx.db
        .resources_regen_loop_timer()
        .try_insert(ResourcesRegenLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(params.resources_regen_tick_millis as u64).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
/// Whenever a resource gets depleted, respawn it.
fn resources_regen(ctx: &ReducerContext, _timer: ResourcesRegenLoopTimer) -> Result<(), String> {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        return Err("Unauthorized access to resources_regen".into());
    }

    if !agents::should_run(ctx) {
        return Ok(());
    }

    let params = ctx.db.parameters_desc().version().find(&0).unwrap();

    let resources_log = ctx.db.resources_log().version().find(&0);

    if resources_log.is_none() {
        return Ok(());
    }

    let resources_log = resources_log.unwrap();

    let mut terrain_cache = TerrainChunkCache::empty();

    // World scale coordinates
    let total_count = (resources_log.world_width as usize) * CHUNK_WIDTH * (resources_log.world_height as usize) * CHUNK_HEIGHT * 9;
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
    let mut missing_resources: HashMap<i32 /* resource id */, usize /* missing count */> = HashMap::with_capacity(32);
    let mut missing_resources_stats: HashMap<i32 /* resource id */, (i32 /* target count */, i32 /* current count */)> =
        HashMap::with_capacity(32);
    for resource in &resources_log.resources {
        let target = (resource.world_target as f32 * 0.95) as i32; //Only respawn resources once their count drops below 95%
        if let Some(ResourceCount {
            num_in_world: resource_count,
            ..
        }) = ctx.db.resource_count().resource_id().find(&resource.resource_id)
        {
            if let Some(resource_desc) = resource_desc.get(&resource.resource_id) {
                if !resource_desc.not_respawning {
                    if resource_count < target {
                        missing_resources.insert(resource.resource_id, (target - resource_count) as usize);
                        missing_resources_stats.insert(resource.resource_id, (target, resource_count));
                        log::info!(
                            "Resource {} (id: {}) - [{}/{}] - {:.2}%",
                            resource_desc.name,
                            resource.resource_id,
                            resource_count,
                            resource.world_target,
                            (resource_count as f32 / resource.world_target as f32) * 100f32
                        )
                    }
                } else {
                    log::info!("Skipping {} because it does not respawn.", resource_desc.name)
                }
            }
        }
    }

    let mut resources_to_delete: Vec<(u64 /* entity id */, i32 /* resource id */)> = Vec::with_capacity(512);
    if missing_resources.len() != 0 {
        // DAB Note: this could be cached in a table when we load static data
        let mut req_resources_in_clumps: HashMap<
            i32, /* clump id */
            HashMap<i32 /* resource id */, usize /* resource count in clump */>,
        > = HashMap::with_capacity(32);
        let mut clump_info_lookup: HashMap<i32 /* clump id */, &ResourceClumpInfo> = HashMap::with_capacity(32);
        for clump_info in &resources_log.resource_clumps {
            let clump_desc = resource_clumps.get(&clump_info.clump_id).unwrap();
            if clump_desc.resource_id.iter().any(|r| !missing_resources.contains_key(r)) {
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

        let mut attempts_left = params.resources_regen_loops;
        let mut removed_clump_ids = Vec::with_capacity(16);

        // Precompute the set of tiles occupied by claims, so that resources can't spawn on them.
        // TODO: Determine whether it would be faster to use a `LazyMemoOccupiedTiles` here.
        let mut occupied_tiles_hashes = PrecomputeOccupiedTiles::new(ctx);

        while req_resources_in_clumps.len() > 0 {
            if attempts_left == 0 {
                break;
            }
            attempts_left = attempts_left - 1;

            // attempt to place each clump into the map in worldgen order until no more resource exists.
            for req in &req_resources_in_clumps {
                let req_resources = req.1;
                let mut possible_spawns = i32::MAX;
                for (res_id, count) in req_resources {
                    possible_spawns = possible_spawns.min((missing_resources[res_id] / *count) as i32);
                }
                if possible_spawns <= 0 {
                    removed_clump_ids.push(*req.0);
                    continue;
                }

                let clump_info = *clump_info_lookup.get(req.0).unwrap();
                let clump_desc = clump_desc_extended.get(&clump_info.clump_id).unwrap();

                // attempt spawning the clump at a random tile in the map
                let resource_spawn_iterations = 1 + possible_spawns / 10; // attempt spawning 10% of the missing resources

                for _ in 0..resource_spawn_iterations {
                    let tile_index = ctx.rng().gen_range(0..total_count - 1) as usize;
                    let offset_coordinates = get_coordinates(tile_index);
                    let hex_coordinates = SmallHexTile::from(offset_coordinates);
                    if try_spawn_resource(
                        ctx,
                        &mut terrain_cache,
                        clump_info,
                        &clump_desc,
                        hex_coordinates,
                        &mut occupied_tiles_hashes,
                        //false,
                        &resource_desc,
                        &mut resources_to_delete,
                        &mut result,
                    ) {
                        //log::info!("---- Regen success");
                        for (res_id, count) in req_resources {
                            let entry = missing_resources.get_mut(&res_id).unwrap();
                            *entry -= *count;
                        }
                    }
                }
            }
            // remove clumps id gathered from clumps lacking resources
            for clump_id in &removed_clump_ids {
                req_resources_in_clumps.remove(&clump_id);
            }
            removed_clump_ids.clear();
        }
    }

    if result.len() > 0 {
        let resources_to_delete_count = resources_to_delete.iter().group_by_and_count(|r| r.1);
        for (entity_id, resource_id) in resources_to_delete {
            ResourceState::despawn(ctx, entity_id, resource_id);
        }

        for (resource_id, delete_count) in resources_to_delete_count {
            let mut count = ctx
                .db
                .resource_count()
                .resource_id()
                .find(&resource_id)
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

        for (resource_id, (resource_name, count)) in spawn_hashmap {
            let previous_count = missing_resources_stats[&resource_id].1;
            let target = missing_resources_stats[&resource_id].0;
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

/// Trait for ways of determining whether a tile is valid as the location for a resource respawn.
///
/// Resources are not allowed to spawn on claimed tiles,
/// so implementors of this trait should mark claimed tiles as occupied.
///
/// [`crate::handlers::resource::respawn_resource_in_chunk`] runs frequently and respawns a single resource at a time,
/// whereas [`resources_regen`] runs infrequently and respawns many resources at once.
/// These have very different needs for how to check if tiles are occupied;
/// it's beneficial for `resources_regen` to do a full scan
/// and pre-load the locations of every `claim_tile_state` row ahead-of-time,
// (pgoldman 2025-06-25 I have not actually measured this,
// but it's what the code did before I got my hands on it and I see no reason to change it)
/// but `respawn_resource_in_chunk` will inspect at most a handful of tiles, and so should look them up on the fly.
///
/// This trait abstracts over different ways of computing the set of occupied tiles.
pub trait OccupiedTiles {
    /// Test whether a tile is occupied.
    ///
    /// If `false`, it may be valid to spawn a resource on that tile.
    fn is_tile_occupied(&mut self, ctx: &ReducerContext, tile_hashcode: i64) -> bool;

    /// Mark that `tile_hashcode` is no longer valid for resource spawning,
    /// as a resource has just been spawned there.
    fn mark_tile_occupied(&mut self, tile_hashcode: i64);
}

/// [`OccupiedTiles`] tracker used by [`resources_regen`],
/// which iterates over the entire set of `claim_tile_state`s ahead of time
/// and stores the set of all claimed tiles.
pub struct PrecomputeOccupiedTiles {
    /// Set of tile hashcodes which are occupied.
    ///
    /// Use `HashSet::contains` to test whether a tile is occupied or not.
    occupied: HashSet<i64>,
}

impl PrecomputeOccupiedTiles {
    /// Construct a new [`PrecomputeOccupiedTiles`], marking every tile within a claim as occupied.
    ///
    /// Resources are not allowed to spawn on claimed tiles,
    /// so any tile within a claim is marked as occupied.
    pub fn new(ctx: &ReducerContext) -> Self {
        let occupied = ctx
            .db
            .claim_tile_state()
            .iter()
            .map(|l| {
                ctx.db
                    .location_state()
                    .entity_id()
                    .find(l.entity_id)
                    .unwrap()
                    .coordinates()
                    .hashcode()
            })
            .collect();
        Self { occupied }
    }

    pub fn place_holder() -> Self {
        Self { occupied: HashSet::new() }
    }
}

impl OccupiedTiles for PrecomputeOccupiedTiles {
    fn is_tile_occupied(&mut self, _ctx: &ReducerContext, tile_hashcode: i64) -> bool {
        self.occupied.contains(&tile_hashcode)
    }
    fn mark_tile_occupied(&mut self, tile_hashcode: i64) {
        self.occupied.insert(tile_hashcode);
    }
}

#[derive(Default)]
/// [`OccupiedTiles`] tracker used by [`crate::game::handlers::resource::respawn_resource_in_chunk`],
/// where queries are performed lazily as needed.
pub struct LazyMemoOccupiedTiles {
    /// Map from tile hashcode to whether it is occupied.
    ///
    /// Stores explicit `false` for tiles which have already been queried and found empty.
    /// No entry means that the tile has not yet been queried.
    occupied: HashMap<i64, bool>,
}

impl OccupiedTiles for LazyMemoOccupiedTiles {
    fn is_tile_occupied(&mut self, ctx: &ReducerContext, tile_hashcode: i64) -> bool {
        *self.occupied.entry(tile_hashcode).or_insert_with(|| {
            let offset_coordinates_small = OffsetCoordinatesSmall::from_hashcode(tile_hashcode);
            let small_hex_tile = SmallHexTile::from(&offset_coordinates_small);
            LocationState::select_all(ctx, &small_hex_tile)
                // Resources are not allowed to spawn on claimed tiles,
                // so any tile within a claim is marked as occupied.
                .map(|location_state| location_state.entity_id)
                .any(|entity_id| ctx.db.claim_tile_state().entity_id().find(entity_id).is_some())
        })
    }

    fn mark_tile_occupied(&mut self, tile_hashcode: i64) {
        self.occupied.insert(tile_hashcode, true);
    }
}

// return value: (Spawn data, resource entities to delete)
pub fn try_spawn_resource_multiple_attempts(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    clump_info: &ResourceClumpInfo,
    hex_coordinates: Vec<SmallHexTile>,
) -> (Vec<(i32, SmallHexTileMessage, i32)>, Vec<u64>, usize) {
    let clump_desc = ctx.db.resource_clump_desc().id().find(&clump_info.clump_id).unwrap();
    let resource_desc: HashMap<i32, ResourceDesc> = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();
    let clump_desc_extended = ResourceClumpDescExtended::new(clump_desc, &resource_desc);

    let mut resources_to_delete = Vec::new();
    let mut added_resources = Vec::new();

    for i in 0..hex_coordinates.len() {
        let coord = hex_coordinates[i];
        if try_spawn_resource(
            ctx,
            terrain_cache,
            clump_info,
            &clump_desc_extended,
            coord,
            occupied_tiles_hashes,
            &resource_desc,
            &mut resources_to_delete,
            &mut added_resources,
        ) {
            return (added_resources, resources_to_delete.iter().map(|r| r.0).collect(), i + 1);
        }
    }
    (
        added_resources,
        resources_to_delete.iter().map(|r| r.0).collect(),
        hex_coordinates.len() + 1,
    )
}

pub fn try_spawn_resource(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    clump_info: &ResourceClumpInfo,
    clump_desc: &ResourceClumpDescExtended,
    hex_coordinates: SmallHexTile,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    //ignore_chances: bool,
    resource_desc: &HashMap<i32, ResourceDesc>,
    resources_to_delete: &mut Vec<(u64 /* entity id */, i32 /* resource id */)>,
    added_resources: &mut Vec<(
        i32,          /*resource id*/
        SmallHexTile, /*spawn coordinates*/
        i32,          /*direction*/
    )>,
) -> bool {
    try_spawn_resource_options(
        ctx,
        terrain_cache,
        clump_info,
        clump_desc,
        hex_coordinates,
        occupied_tiles_hashes,
        resource_desc,
        resources_to_delete,
        added_resources,
        false,
        false,
    )
}

pub fn try_spawn_resource_options(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    clump_info: &ResourceClumpInfo,
    clump_desc: &ResourceClumpDescExtended,
    hex_coordinates: SmallHexTile,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    //ignore_chances: bool,
    resource_desc: &HashMap<i32, ResourceDesc>,
    resources_to_delete: &mut Vec<(u64 /* entity id */, i32 /* resource id */)>,
    added_resources: &mut Vec<(
        i32,          /*resource id*/
        SmallHexTile, /*spawn coordinates*/
        i32,          /*direction*/
    )>,
    ignore_biome: bool,
    use_spawn_chance: bool,
) -> bool {
    // Check center node (footprint, elevation, water, noise, biome, etc.)
    let mut max_elevation_delta = 0;
    if !is_valid_resource_node_options(
        ctx,
        terrain_cache,
        clump_info,
        hex_coordinates,
        &mut max_elevation_delta,
        //ignore_chances,
        ignore_biome,
        use_spawn_chance,
    ) {
        return false;
    }

    try_spawn_resource_no_node_validation(
        ctx,
        terrain_cache,
        max_elevation_delta,
        clump_desc,
        hex_coordinates,
        occupied_tiles_hashes,
        resource_desc,
        resources_to_delete,
        added_resources,
    )
}

pub fn try_spawn_resource_no_clump_info(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    spawns_on_land: bool,
    clump_desc: &ResourceClumpDescExtended,
    hex_coordinates: SmallHexTile,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    resource_desc: &HashMap<i32, ResourceDesc>,
    resources_to_delete: &mut Vec<(u64 /* entity id */, i32 /* resource id */)>,
    added_resources: &mut Vec<(
        i32,          /*resource id*/
        SmallHexTile, /*spawn coordinates*/
        i32,          /*direction*/
    )>,
) -> bool {
    let is_submerged = game_state_filters::is_submerged(ctx, terrain_cache, hex_coordinates);
    if spawns_on_land && is_submerged || !spawns_on_land && !is_submerged {
        return false;
    }

    try_spawn_resource_no_node_validation(
        ctx,
        terrain_cache,
        0, // prizes are not allowed on non flat tiles
        clump_desc,
        hex_coordinates,
        occupied_tiles_hashes,
        resource_desc,
        resources_to_delete,
        added_resources,
    )
}

pub fn try_spawn_resource_no_node_validation(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    max_elevation_delta: i32,
    clump_desc: &ResourceClumpDescExtended,
    hex_coordinates: SmallHexTile,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    //ignore_chances: bool,
    resource_desc: &HashMap<i32, ResourceDesc>,
    resources_to_delete: &mut Vec<(u64 /* entity id */, i32 /* resource id */)>,
    added_resources: &mut Vec<(
        i32,          /*resource id*/
        SmallHexTile, /*spawn coordinates*/
        i32,          /*direction*/
    )>,
) -> bool {
    // Check center node (footprint, elevation, water, noise, biome, etc.)
    let mut valid = false;
    let mut facing_direction;
    if clump_desc.is_single_tile {
        //Single-tile resources don't need to spin
        facing_direction = HexDirection::FLAT[0];
        valid = is_valid_resource_footprint(
            ctx,
            terrain_cache,
            hex_coordinates,
            &clump_desc.footprints,
            facing_direction,
            clump_desc.spawn_priority,
            max_elevation_delta,
            occupied_tiles_hashes,
            resource_desc,
        );
    } else {
        // Attempt to place the resource by picking a random starting direction and looking at the footprints
        facing_direction = HexDirection::FLAT[ctx.rng().gen_range(0..HexDirection::FLAT.len()) as usize];
        for _ in 0..HexDirection::FLAT.len() {
            if is_valid_resource_footprint(
                ctx,
                terrain_cache,
                hex_coordinates,
                &clump_desc.footprints,
                facing_direction,
                clump_desc.spawn_priority,
                max_elevation_delta,
                occupied_tiles_hashes,
                resource_desc,
            ) {
                valid = true;
                break;
            }

            // If failed, rotate and try again
            facing_direction = HexDirection::next_flat(facing_direction);
        }
    }

    if !valid {
        return false;
    }

    let default_footprint = vec![FootprintTile {
        x: 0,
        z: 0,
        footprint_type: FootprintType::WalkableResource,
    }];

    for i in 0..clump_desc.clump.resource_id.len() {
        let resource_id = clump_desc.clump.resource_id[i];
        let offset_x = clump_desc.clump.x[i];
        let offset_z = clump_desc.clump.z[i];
        let resource_dir = clump_desc.clump.direction[i];
        let resource_desc = clump_desc.resource_desc[i];

        let mut res_footprint = &resource_desc.footprint;
        if res_footprint.len() == 0 {
            // default: single-tile hitbox resource
            res_footprint = &default_footprint
        }
        let rotate_steps = (facing_direction as i32) / 2;
        for res_footprint_delta in res_footprint.iter().filter(|f| f.footprint_type != FootprintType::Perimeter) {
            let mut delta = res_footprint_delta.clone();
            let is_center = res_footprint_delta.x == 0 && res_footprint_delta.z == 0;
            delta.x += offset_x;
            delta.z += offset_z;
            let taken_coordinates = SmallHexTile {
                x: hex_coordinates.x + delta.x,
                z: hex_coordinates.z + delta.z,
                dimension: hex_coordinates.dimension,
            }
            .rotate_around(&hex_coordinates, rotate_steps);
            let direction = if resource_dir == -1 {
                HexDirection::FLAT[ctx.rng().gen_range(0..HexDirection::FLAT.len()) as usize] as i32
            } else {
                ((rotate_steps + resource_dir) % 6) * 2
            };

            if is_center {
                added_resources.push((resource_id, taken_coordinates, direction));
            }
            occupied_tiles_hashes.mark_tile_occupied(taken_coordinates.hashcode());

            let footprints = FootprintTileState::get_at_location(ctx, &taken_coordinates);
            for footprint in footprints {
                if ctx.db.building_state().entity_id().find(&footprint.owner_entity_id).is_some() {
                    return false;
                }
                if let Some(resource) = ctx.db.resource_state().entity_id().find(&footprint.owner_entity_id) {
                    resources_to_delete.push((footprint.owner_entity_id, resource.resource_id));
                }
            }
        }
    }
    true
}

fn is_valid_resource_node_options(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    resource_clump_info: &ResourceClumpInfo,
    hex_coordinates: SmallHexTile,
    max_elevation_delta: &mut i32,
    ignore_biome: bool,
    use_spawn_chance: bool,
    //ignore_chances: bool,
) -> bool {
    let terrain_coordinates = hex_coordinates.parent_large_tile();
    let terrain_offset = OffsetCoordinatesLarge::from(terrain_coordinates);
    let terrain = match terrain_cache.get_terrain_cell(ctx, &terrain_coordinates) {
        Some(t) => t,
        None => return false,
    };
    let water_level = terrain.water_level;
    let elevation = terrain.elevation;

    for info in &resource_clump_info.spawn_info {
        if use_spawn_chance && info.chance < 1f32 && ctx.rng().gen_range(0f32..1f32) > info.chance {
            continue;
        }

        // DAB Note: On the client those checks are not per biome, so they should be serialized differently.
        // For this reason, we can return FALSE if any of those checks fail since they will contain the same data.
        if !is_valid_land_or_water(
            info.spawns_on_land,
            info.land_elevation_min,
            info.land_elevation_max,
            info.spawns_in_water,
            info.water_depth_min,
            info.water_depth_max,
            elevation,
            water_level,
        ) {
            return false;
        }

        let mut biome_value = 1.0f32;
        if !ignore_biome {
            biome_value = 0.0f32;
            let biome_index = info.biome_index;
            for j in 0..4 {
                let biome = (terrain.biomes >> (j * 8)) & 0xFF;
                if (biome as u64) == biome_index {
                    let density = ((terrain.biome_density >> (j * 8)) & 0xFF) as f32;
                    biome_value = density / 128.0;
                    if biome_value > 0.0 {
                        break;
                    }
                }
            }
            if biome_value <= 0.0 {
                continue;
            }
        }

        let noise = noise_helper::get(
            Vector2 {
                x: terrain_offset.x as f32,
                y: terrain_offset.z as f32,
            },
            info.noise_scale,
            info.noise_octaves,
            info.noise_persistance,
            info.noise_lacunarity,
            Vector2 {
                x: info.noise_offset_x,
                y: info.noise_offset_y,
            },
        );

        if (noise < info.noise_threshold_bottom) | (noise > info.noise_threshold_top) {
            continue;
        }

        if ctx.rng().gen_range(0f32..1f32) < biome_value {
            //|| ignore_chances {
            *max_elevation_delta = info.max_elevation_delta;
            return true;
        }
    }
    false
}

pub fn is_valid_single_resource_spawn(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    resource_desc: &ResourceDesc,
    center_coordinates: SmallHexTile,
    direction_index: i32,
) -> bool {
    let terrain = match terrain_cache.get_terrain_cell(ctx, &center_coordinates.parent_large_tile()) {
        Some(t) => t,
        None => return false,
    };

    if !is_valid_land_or_water(
        resource_desc.spawns_on_land,
        resource_desc.land_elevation_min as i16,
        resource_desc.land_elevation_max as i16,
        resource_desc.spawns_in_water,
        resource_desc.water_depth_min as i16,
        resource_desc.water_depth_max as i16,
        terrain.elevation,
        terrain.water_level,
    ) {
        return false;
    }

    let footprint = if resource_desc.footprint.is_empty() {
        vec![FootprintTile {
            x: 0,
            z: 0,
            footprint_type: FootprintType::Hitbox,
        }]
    } else {
        resource_desc
            .footprint
            .iter()
            .filter(|tile| tile.footprint_type != FootprintType::Perimeter)
            .cloned()
            .collect()
    };

    let mut occupied_tiles = LazyMemoOccupiedTiles::default();
    is_valid_single_resource_footprint(
        ctx,
        terrain_cache,
        center_coordinates,
        &footprint,
        HexDirection::from(direction_index),
        resource_desc.max_elevation_delta,
        &mut occupied_tiles,
    )
}

fn is_valid_land_or_water(
    spawns_on_land: bool,
    land_elevation_min: i16,
    land_elevation_max: i16,
    spawns_in_water: bool,
    water_depth_min: i16,
    water_depth_max: i16,
    elevation: i16,
    water_level: i16,
) -> bool {
    let is_submerged = water_level > elevation;
    let water_depth = water_level - elevation;

    if spawns_on_land && !is_submerged && elevation >= land_elevation_min && elevation <= land_elevation_max {
        return true;
    }
    if spawns_in_water && is_submerged && water_depth >= water_depth_min && water_depth <= water_depth_max {
        return true;
    }

    false
}

fn is_valid_resource_footprint(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    center_coordinates: SmallHexTile,
    footprint_no_perimiter: &Vec<FootprintTile>,
    direction: HexDirection,
    spawn_priority: i32,
    max_elevation_delta: i32,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
    resource_desc: &HashMap<i32, ResourceDesc>,
) -> bool {
    let mut min_elevation = i16::MAX;
    let mut max_elevation = i16::MIN;

    for delta in footprint_no_perimiter {
        let footprint_coordinates = (SmallHexTile {
            x: center_coordinates.x + delta.x,
            z: center_coordinates.z + delta.z,
            dimension: center_coordinates.dimension,
        })
        .rotate_around(&center_coordinates, (direction as i32) / 2);

        // Something was already elected to spawn on that tile in this batch
        if occupied_tiles_hashes.is_tile_occupied(ctx, footprint_coordinates.hashcode()) {
            return false;
        }

        if !is_valid_cell(ctx, footprint_coordinates, spawn_priority, resource_desc) {
            return false;
        }

        let footprint_terrain = match terrain_cache.get_terrain_cell(ctx, &footprint_coordinates.parent_large_tile()) {
            Some(t) => t,
            None => return false,
        };

        let footprint_elevation = footprint_terrain.elevation;
        if footprint_elevation == -1 {
            return false;
        }

        let elevations: [i16; 3] = footprint_coordinates
            .get_terrain_coordinates()
            .map(|c| terrain_cache.get_terrain_cell(ctx, &c).unwrap_or_default().elevation);
        let min = *elevations.iter().min().unwrap();
        let max = *elevations.iter().max().unwrap();
        min_elevation = min_elevation.min(min);
        max_elevation = max_elevation.max(max);

        if min_elevation == -1 {
            return false;
        }

        if max_elevation - min_elevation > max_elevation_delta as i16 {
            return false;
        }
    }

    return true;
}

fn is_valid_single_resource_footprint(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    center_coordinates: SmallHexTile,
    footprint_no_perimiter: &Vec<FootprintTile>,
    direction: HexDirection,
    max_elevation_delta: i32,
    occupied_tiles_hashes: &mut impl OccupiedTiles,
) -> bool {
    let mut min_elevation = i16::MAX;
    let mut max_elevation = i16::MIN;

    for delta in footprint_no_perimiter {
        let footprint_coordinates = (SmallHexTile {
            x: center_coordinates.x + delta.x,
            z: center_coordinates.z + delta.z,
            dimension: center_coordinates.dimension,
        })
        .rotate_around(&center_coordinates, (direction as i32) / 2);

        if occupied_tiles_hashes.is_tile_occupied(ctx, footprint_coordinates.hashcode()) {
            return false;
        }

        for loc in LocationState::select_all(ctx, &footprint_coordinates) {
            let entity_id = loc.entity_id;

            if let Some(fp) = ctx.db.footprint_tile_state().entity_id().find(&entity_id) {
                if ((fp.footprint_type == FootprintType::Hitbox) | (fp.footprint_type == FootprintType::Walkable))
                    && ctx.db.building_state().entity_id().find(&fp.owner_entity_id).is_some()
                {
                    return false;
                }
                if ctx.db.resource_state().entity_id().find(fp.owner_entity_id).is_some() {
                    return false;
                }
            }

            if ctx.db.paved_tile_state().entity_id().find(&entity_id).is_some() {
                return false;
            }
        }

        let footprint_terrain = match terrain_cache.get_terrain_cell(ctx, &footprint_coordinates.parent_large_tile()) {
            Some(t) => t,
            None => return false,
        };

        let footprint_elevation = footprint_terrain.elevation;
        if footprint_elevation == -1 {
            return false;
        }

        let elevations: [i16; 3] = footprint_coordinates
            .get_terrain_coordinates()
            .map(|c| terrain_cache.get_terrain_cell(ctx, &c).unwrap_or_default().elevation);
        let min = *elevations.iter().min().unwrap();
        let max = *elevations.iter().max().unwrap();
        min_elevation = min_elevation.min(min);
        max_elevation = max_elevation.max(max);

        if min_elevation == -1 {
            return false;
        }

        if max_elevation - min_elevation > max_elevation_delta as i16 {
            return false;
        }
    }

    true
}

fn is_valid_cell(
    ctx: &ReducerContext,
    hex_coordinates: SmallHexTile,
    spawn_priority: i32,
    resource_desc: &HashMap<i32, ResourceDesc>,
) -> bool {
    for loc in LocationState::select_all(ctx, &hex_coordinates) {
        let entity_id = loc.entity_id;

        // footprint must not be overlapping a building or higher priority resource
        if let Some(fp) = ctx.db.footprint_tile_state().entity_id().find(&entity_id) {
            if ((fp.footprint_type == FootprintType::Hitbox) | (fp.footprint_type == FootprintType::Walkable))
                && ctx.db.building_state().entity_id().find(&fp.owner_entity_id).is_some()
            {
                return false;
            }
            if let Some(deposit) = ctx.db.resource_state().entity_id().find(fp.owner_entity_id) {
                if let Some(resource_desc) = resource_desc.get(&deposit.resource_id) {
                    if !resource_desc.flattenable || resource_desc.spawn_priority >= spawn_priority {
                        return false;
                    }
                }
            }
        }

        // Don't spawn over paving
        if ctx.db.paved_tile_state().entity_id().find(&entity_id).is_some() {
            return false;
        }
    }

    // should we prevent a resource from spawning over a cargo or loot bag?
    true
}

#[derive(Debug)]
pub struct ResourceClumpDescExtended<'a> {
    pub clump: ResourceClumpDesc,
    pub resource_desc: Vec<&'a ResourceDesc>,
    pub footprints: Vec<FootprintTile>,
    pub spawn_priority: i32,
    pub is_single_tile: bool,
}

impl<'a> ResourceClumpDescExtended<'a> {
    pub fn new(clump: ResourceClumpDesc, resource_desc_cache: &'a HashMap<i32, ResourceDesc>) -> Self {
        let resource_desc: Vec<&ResourceDesc> = clump.resource_id.iter().map(|r| resource_desc_cache.get(r).unwrap()).collect();
        let spawn_priority = resource_desc.iter().map(|r| r.spawn_priority).max().unwrap();

        let mut footprints_no_perimiter = Vec::new();
        for i in 0..clump.resource_id.len() {
            let x = clump.x[i];
            let z = clump.z[i];
            let resource_desc = resource_desc[i];
            if resource_desc.footprint.len() == 0 {
                // default: (0,0)
                footprints_no_perimiter.push(FootprintTile {
                    x,
                    z,
                    footprint_type: FootprintType::Hitbox,
                }) // what should be the type? is it important for world generation?
            } else {
                for delta in &resource_desc.footprint {
                    let mut d = delta.clone();
                    d.x += x;
                    d.z += z;
                    footprints_no_perimiter.push(d);
                }
            }
        }
        let is_single_tile = footprints_no_perimiter.len() == 0
            || (footprints_no_perimiter.len() == 1 && footprints_no_perimiter[0].x == 0 && footprints_no_perimiter[0].z == 0);
        footprints_no_perimiter.retain(|f| f.footprint_type != FootprintType::Perimeter);

        Self {
            clump,
            resource_desc,
            footprints: footprints_no_perimiter,
            spawn_priority,
            is_single_tile,
        }
    }

    pub fn place_holder(clump: ResourceClumpDesc) -> Self {
        Self {
            clump,
            resource_desc: Vec::new(),
            footprints: Vec::new(),
            spawn_priority: 0,
            is_single_tile: true,
        }
    }
}
