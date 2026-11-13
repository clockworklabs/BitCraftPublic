use spacetimedb::{log, ReducerContext, SpacetimeType, Table};

use crate::{
    game::{coordinates::*, dimensions, game_state::unix_ms, unity_helpers::common_rng::CommonRNG},
    resource_clump_desc, resource_desc,
};

use super::{
    world_definition::{self, WorldDefinition},
    world_generation::world_graph::WorldGraph,
};

const CHUNK_WIDTH: u64 = world_definition::TERRAIN_CHUNK_WIDTH as u64;
const CHUNK_HEIGHT: u64 = world_definition::TERRAIN_CHUNK_HEIGHT as u64;

#[spacetimedb::table(accessor = resources_log)]
pub struct ResourcesLog {
    #[primary_key]
    pub version: i32,

    pub world_width: u64,
    pub world_height: u64,
    pub resource_clumps: Vec<ResourceClumpInfo>,
    pub resources: Vec<ResourceInfo>,

    pub random: CommonRNG,
}

#[spacetimedb::table(accessor = single_resource_clump_info)]
pub struct SingleResourceClumpInfo {
    #[primary_key]
    pub clump_id: i32,
    pub resource_clump_info: ResourceClumpInfo,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct ResourceClumpInfo {
    pub clump_id: i32,
    pub spawn_info: Vec<SpawnInfo>,
}

#[derive(SpacetimeType)]
pub struct ResourceInfo {
    pub resource_id: i32,
    pub world_target: i32,
}

#[derive(SpacetimeType, Debug, Clone)]
pub struct SpawnInfo {
    pub biome_index: u64,
    pub chance: f32,
    pub noise_offset_x: f32,
    pub noise_offset_y: f32,
    pub noise_threshold_bottom: f32,
    pub noise_threshold_top: f32,
    pub noise_scale: f32,
    pub noise_octaves: i32,
    pub noise_persistance: f32,
    pub noise_lacunarity: f32,
    pub spawns_on_land: bool,
    pub land_elevation_min: i16,
    pub land_elevation_max: i16,
    pub spawns_in_water: bool,
    pub water_depth_min: i16,
    pub water_depth_max: i16,
    pub max_elevation_delta: i32,
}

impl ResourcesLog {
    pub fn populate_single_resource_chunks_info(ctx: &ReducerContext) {
        for clump_info in &ctx.db.resources_log().version().find(0).unwrap().resource_clumps {
            // NOT using SingleResourceToClumpDesc since more than 1 clump can have a similar single resource (in which case only 1 clump is added there
            // and it might not be the one we are looking for)
            if let Some(clump) = ctx.db.resource_clump_desc().id().find(clump_info.clump_id) {
                if clump.resource_id.len() == 1 {
                    if let Err(err) = ctx.db.single_resource_clump_info().try_insert(SingleResourceClumpInfo {
                        clump_id: clump_info.clump_id,
                        resource_clump_info: clump_info.clone(),
                    }) {
                        let id = clump_info.clump_id;
                        let err_msg = format!("Couldn't insert SingleResourceClumpInfo record with id {id}. Error message: {err}");
                        log::error!("{}", err_msg);
                    }
                }
            }
        }
        log::info!("Imported clump info from resource logs");
    }

    pub fn save(ctx: &ReducerContext, world_graph: &WorldGraph, world_definition: &WorldDefinition) {
        log::info!("Saving ResourcesLog");

        let world_width = world_definition.size.x as usize;
        if world_width == 0 {
            return;
        }
        let world_height = world_definition.size.y as usize;
        if world_height == 0 {
            return;
        }

        let resource_definitions = &world_definition.resources_map.resources;

        let mut resources: Vec<ResourceInfo> = ctx
            .db
            .resource_desc()
            .iter()
            .map(|r| ResourceInfo {
                resource_id: r.id,
                world_target: 0,
            })
            .collect();

        let mut resource_clumps: Vec<ResourceClumpInfo> = vec![];
        for resource_definition in resource_definitions {
            let clump_id = resource_definition.resource_details.clump_id;
            let mut spawn_details: Vec<SpawnInfo> = vec![];
            for biome in &resource_definition.biomes {
                if biome.chance <= 0f32 {
                    continue;
                }

                let noise = &biome.noise_specs;
                let offset = &biome.noise_specs.offset;
                let threshold = &biome.noise_threshold;

                spawn_details.push(SpawnInfo {
                    biome_index: biome.biome_index as u64,
                    chance: biome.chance,
                    noise_offset_x: offset.x,
                    noise_offset_y: offset.y,
                    noise_threshold_bottom: threshold.x,
                    noise_threshold_top: threshold.y,
                    noise_scale: noise.scale,
                    noise_octaves: noise.octaves,
                    noise_persistance: noise.persistance,
                    noise_lacunarity: noise.lacunarity,
                    spawns_on_land: resource_definition.resource_details.spawns_on_land,
                    land_elevation_min: resource_definition.resource_details.land_elevation_range.x as i16,
                    land_elevation_max: resource_definition.resource_details.land_elevation_range.y as i16,
                    spawns_in_water: resource_definition.resource_details.spawns_in_water,
                    water_depth_min: resource_definition.resource_details.water_depth_range.x as i16,
                    water_depth_max: resource_definition.resource_details.water_depth_range.y as i16,
                    max_elevation_delta: resource_definition.resource_details.max_elevation_delta,
                })
            }

            resource_clumps.push(ResourceClumpInfo {
                clump_id,
                spawn_info: spawn_details,
            });
        }

        // This matches the sampling in resources_regen.rs agent to avoid disparities
        let entities_graph = &world_graph.entities_graph;
        for chunk_x in 0..world_width as u64 {
            for chunk_y in 0..world_height as u64 {
                let start_x = chunk_x * CHUNK_WIDTH;
                let start_y = chunk_y * CHUNK_HEIGHT;
                let end_x = start_x + CHUNK_WIDTH;
                let end_y = start_y + CHUNK_HEIGHT;

                let start = LargeHexTile::from(OffsetCoordinatesLarge {
                    x: start_x as i32,
                    z: start_y as i32,
                    dimension: dimensions::OVERWORLD,
                })
                .center_small_tile()
                .to_offset_coordinates();
                let end = LargeHexTile::from(OffsetCoordinatesLarge {
                    x: end_x as i32,
                    z: end_y as i32,
                    dimension: dimensions::OVERWORLD,
                })
                .center_small_tile()
                .to_offset_coordinates();

                let start_x = start.x as u64;
                let start_y = start.z as u64;
                let end_x = end.x as u64;
                let end_y = end.z as u64;

                let width = end_x - start_x;
                let height = end_y - start_y;
                let total_count = width * height;

                let get_coordinates = |index: u64| {
                    let mut y = index / width;
                    let mut x = index - y * width;

                    x += start_x;
                    y += start_y;

                    OffsetCoordinatesSmall {
                        x: x as i32,
                        z: y as i32,
                        dimension: dimensions::OVERWORLD,
                    }
                };

                for k in 0..total_count {
                    let offset = get_coordinates(k);
                    let graph_index = entities_graph.get_index_from_offset_coordinates(offset.into());
                    let node = entities_graph.get(graph_index);
                    match node {
                        Some(node) => {
                            if let Some(resource_data) = &node.resource {
                                if let Some(resource_info) = resources.iter_mut().find(|r| r.resource_id == resource_data.resource_id) {
                                    resource_info.world_target += 1;
                                } else if resource_data.resource_id != 0 {
                                    // This probably can't happen
                                    log::error!("Could not find resource {}", resource_data.resource_id);
                                }
                            };
                        }
                        None => continue,
                    }
                }
            }
        }

        let log = Self {
            version: 0,

            world_width: world_width as u64,
            world_height: world_height as u64,
            resource_clumps,
            resources,

            random: CommonRNG::from_seed(unix_ms(ctx.timestamp) as i32),
        };

        if ctx.db.resources_log().try_insert(log).is_err() {
            log::error!("Failed to insert resource log");
        }
    }
}
