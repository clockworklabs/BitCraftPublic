use crate::game::coordinates::offset_coordinates::OffsetCoordinates;
use crate::messages::components::*;
use crate::messages::game_util::DimensionType;

use super::world_generator::GeneratedWorld;

const WATER_LEVEL: i16 = 10;
const ZONING_TYPE_PLAYER_START_CELL: u8 = 1;

pub fn generate() -> GeneratedWorld {
    let world_chunk_width: i32 = 5;
    let world_chunk_height: i32 = 5;

    let height = (TerrainChunkState::HEIGHT as i32) * world_chunk_height;
    let width = (TerrainChunkState::WIDTH as i32) * world_chunk_width;

    let mut generated_world = GeneratedWorld {
        chunks: vec![],
        buildings: vec![],
        deposits: vec![],
        enemies: vec![],
        dropped_inventories: vec![],
        npcs: vec![],
        ignore_claim_creation: false,
        dimensions: vec![DimensionDescriptionState {
            entity_id: 1,
            dimension_id: 1,

            dimension_type: DimensionType::Overworld,
            interior_instance_id: 0,
            dimension_position_large_x: 0,
            dimension_position_large_z: 0,
            dimension_size_large_x: world_chunk_width as u32,
            dimension_size_large_z: world_chunk_height as u32,
            dimension_network_entity_id: 0,
            collapse_timestamp: 0,
        }],
    };

    for i in 0..world_chunk_width {
        generated_world.chunks.push(Vec::new());
        for j in 0..world_chunk_height {
            let mut chunk = TerrainChunkState::default_with_capacity();
            chunk.chunk_x = i;
            chunk.chunk_z = j;
            chunk.chunk_index = (j * 1000 + i + 1) as u64; // 1000 is over the maximum chunk size and will skip a table access at runtime
            generated_world.chunks[i as usize].push(chunk);
        }
    }

    for x in 0..height {
        for z in 0..width {
            let offset = OffsetCoordinates { x, z, dimension: 1 };

            let elevation = 20;
            let biomes = 0; // Dev Biome
            let biome_density = 30;

            let zoning_type = if x == height / 2 && z == width / 2 {
                ZONING_TYPE_PLAYER_START_CELL
            } else {
                0
            };

            let terrain_cell = TerrainCell {
                x: offset.x,
                z: offset.z,
                elevation,
                water_level: WATER_LEVEL,
                zoning_type,
                biomes,
                original_elevation: elevation,
                biome_density,
                ..Default::default()
            };

            let chunk_indices = (
                offset.x / TerrainChunkState::WIDTH as i32,
                offset.z / TerrainChunkState::HEIGHT as i32,
            );

            let chunk = &mut generated_world.chunks[chunk_indices.0 as usize][chunk_indices.1 as usize];
            chunk.set_entity(offset.into(), terrain_cell);
        }
    }

    generated_world
}
