use crate::messages::components::*;
use crate::messages::game_util::*;
use crate::messages::world_gen::*;

use super::super::unity_helpers::vector2int::Vector2Int;
use super::world_generation::world_graph::WorldGraph;

use super::world_definition::{self, WorldDefinition};

const ZONING_TYPE_NONE: u8 = 0;
const ZONING_TYPE_PLAYER_START_CELL: u8 = 1;
const _ZONING_TYPE_TRADING_POST: u8 = 2;
const _ZONING_TYPE_BUILDING_NOT_ALLOWED: u8 = 5;

pub struct GeneratedWorld {
    pub chunks: Vec<Vec<TerrainChunkState>>,
    pub buildings: Vec<WorldGenGeneratedBuilding>,
    pub deposits: Vec<WorldGenGeneratedResourceDeposit>,
    pub enemies: Vec<WorldGenGeneratedEnemy>,
    pub dropped_inventories: Vec<WorldGenGeneratedDroppedInventory>,
    pub npcs: Vec<WorldGenGeneratedNPC>,
    pub dimensions: Vec<DimensionDescriptionState>,
    pub ignore_claim_creation: bool,
}

pub fn generate(world_definition: &WorldDefinition, world_graph: &WorldGraph) -> GeneratedWorld {
    // log::info!("[1/3] Generating graph...");
    // let mut graph = world_hex_graph_generator::generate(world_definition, debug);
    // log::info!("[2/3] Setting resources...");
    // world_hex_graph_generator::set_resources(world_definition, &mut graph);
    // //_dbg_render_graph(world_definition, &graph, "../b.png"); //docker cp bitcraft-game-1:/usr/src/app/b.png .
    // log::info!("[3/3] Generating chunks...");
    generate_chunks(world_definition, world_graph)
}

// fn _dbg_render_graph(
//     world_definition: &WorldDefinition,
//     graph: &HexGraph<TerrainNode>,
//     filename: &str,
// ) {
//     let w = (world_definition.size.x * world_definition::TERRAIN_CHUNK_WIDTH) as usize;
//     let h = (world_definition.size.y * world_definition::TERRAIN_CHUNK_HEIGHT) as usize;

//     let mut v: Vec<u8> = vec![0; (w * h * 4) as usize];
//     for i in 0..graph.count() {
//         let node = graph.get(i).unwrap();
//         let coordinates = node.coordinates;
//         let offset = coordinates.to_offset_coordinates();

//         let x = offset.x as usize;
//         let y = h - 1 - offset.z as usize;
//         let elevation = node.elevation;
//         let water_level = node.water_level;
//         let is_water = water_level > elevation;
//         let is_spawn = is_water
//             && world_definition
//                 .biomes_map
//                 .is_spawn_at_pos(node.world_position());

//         let mut r = 0;
//         let mut g = 0;
//         let mut b = 0;
//         if is_spawn {
//             r = 255;
//             b = 255;
//         } else if world_definition
//             .biomes_map
//             .is_spawn_at_pos(node.world_position())
//         {
//             r = 255;
//         } else if is_water {
//             b = 255;
//         } else {
//             g = (4 * elevation) as u8;
//         }

//         v[((x + y * w) * 4) as usize] = r;
//         v[((x + y * w) * 4) as usize + 1] = g;
//         v[((x + y * w) * 4) as usize + 2] = b;
//         v[((x + y * w) * 4) as usize + 3] = 255;
//     }
//     let imgbuf: &[u8] = v.as_slice();
//     image::save_buffer(
//         filename,
//         imgbuf,
//         w as u32,
//         h as u32,
//         image::ColorType::Rgba8,
//     )
//     .unwrap();
// }

fn generate_chunks(world_definition: &WorldDefinition, graph: &WorldGraph) -> GeneratedWorld {
    let dimensions: Vec<DimensionDescriptionState> = vec![DimensionDescriptionState {
        entity_id: 1,
        dimension_id: 1,
        dimension_type: DimensionType::Overworld,
        interior_instance_id: 0,
        dimension_position_large_x: 0,
        dimension_position_large_z: 0,
        dimension_size_large_x: world_definition.size.x as u32,
        dimension_size_large_z: world_definition.size.y as u32,
        dimension_network_entity_id: 0,
        collapse_timestamp: 0,
    }];

    let biomes_map = &world_definition.biomes_map;

    let chunks_size_x = world_definition.size.x as usize;
    let chunks_size_y = world_definition.size.y as usize;

    let mut chunks: Vec<Vec<TerrainChunkState>> = Vec::new(); // vec![vec![None; chunks_size_y]; chunks_size_x];
    for i in 0..chunks_size_x as i32 {
        chunks.push(Vec::new());
        for j in 0..chunks_size_y as i32 {
            let mut c = TerrainChunkState::default_with_capacity();
            c.chunk_x = i;
            c.chunk_z = j;
            c.chunk_index = (j * 1000 + i + 1) as u64; // 1000 is over the maximum chunk size and will skip a table access at runtime
            chunks[i as usize].push(c);
        }
    }

    let terrain_graph = &graph.terrain_graph;
    for i in 0..terrain_graph.count() {
        let node = terrain_graph.get(i).unwrap();
        let coordinates = node.coordinates;
        let offset = coordinates.to_offset_coordinates();
        let chunk_indices = Vector2Int::new(
            offset.x / world_definition::TERRAIN_CHUNK_WIDTH,
            offset.z / world_definition::TERRAIN_CHUNK_HEIGHT,
        );

        let chunk = &mut chunks[chunk_indices.x as usize][chunk_indices.y as usize];

        let water_body_type = match Biome::to_enum(node.biome() as u8) {
            Biome::Ocean => SurfaceType::OceanBiome,
            Biome::Swamp => SurfaceType::Swamp,
            _ => match node.node_type {
                super::world_generation::terrain_node::NodeType::Sea => SurfaceType::Ocean,
                super::world_generation::terrain_node::NodeType::Lake => SurfaceType::Lake,
                super::world_generation::terrain_node::NodeType::Land => SurfaceType::Ground,
                super::world_generation::terrain_node::NodeType::River => SurfaceType::River,
            },
        } as u8;

        let terrain_cell = TerrainCell {
            x: offset.x,
            z: offset.z,
            elevation: node.elevation,
            water_level: node.water_level,
            water_body_type,
            biomes: node.biomes,
            zoning_type: (if biomes_map.is_spawn_at_pos(node.world_position()) && node.water_level < node.elevation {
                ZONING_TYPE_PLAYER_START_CELL
            } else {
                ZONING_TYPE_NONE
            }),
            original_elevation: node.elevation,
            biome_density: node.biome_density,

            ..Default::default()
        };

        chunk.set_entity(offset.into(), terrain_cell);
    }

    let mut buildings = vec![]; //(╥_╥)
    let mut deposits = vec![]; //(╥_╥)

    let entities_graph = &graph.entities_graph;
    for i in 0..entities_graph.count() {
        let node = entities_graph.get(i).unwrap();
        let coordinates = node.coordinates;

        let offset = coordinates.to_offset_coordinates();

        let building = node.building;
        match building {
            Some(b) => {
                if b.id <= 0 {
                    continue;
                }

                let building_state = WorldGenGeneratedBuilding {
                    x: offset.x,
                    z: offset.z,
                    building: Some(BuildingState {
                        building_description_id: b.id,
                        direction_index: b.direction,

                        ..Default::default()
                    }),
                    dimension: 1,
                };

                buildings.push(building_state);
            }
            None => {
                let resource = node.resource.as_ref();
                match resource {
                    Some(r) => match r.details {
                        Some(_rd) => {
                            let resource_deposit = WorldGenGeneratedResourceDeposit {
                                x: offset.x,
                                z: offset.z,
                                deposit: Some(ResourceState {
                                    resource_id: r.resource_id,
                                    direction_index: r.direction,

                                    ..Default::default()
                                }),
                                dimension: 1,
                            };

                            deposits.push(resource_deposit);
                        }
                        None => continue,
                    },
                    None => continue,
                }
            }
        }
    }

    GeneratedWorld {
        chunks,
        buildings,
        deposits,
        dimensions,
        npcs: vec![],
        dropped_inventories: vec![],
        enemies: vec![],
        ignore_claim_creation: false,
    }
}
