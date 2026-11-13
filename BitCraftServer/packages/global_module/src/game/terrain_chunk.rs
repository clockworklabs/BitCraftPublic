use spacetimedb::ReducerContext;

use super::{coordinates::ChunkCoordinates, dimensions};
use crate::messages::{components::TerrainChunkState, generic::world_region_state, util::ChunkCoordinatesMessage};

impl TerrainChunkState {
    pub const WIDTH: u32 = 32;
    pub const HEIGHT: u32 = 32;

    pub fn chunk_index_from_coords(coords: &ChunkCoordinates) -> u64 {
        coords.chunk_index()
    }

    pub fn chunk_coord_from_chunk_index(chunk_index: u64) -> ChunkCoordinates {
        let chunk_index = chunk_index - 1;
        let dimension = chunk_index / 1000000 + 1;
        let z = (chunk_index - (dimension - 1) * 1000000) / 1000;
        let x = chunk_index - (dimension - 1) * 1000000 - z * 1000;
        ChunkCoordinates {
            x: x as i32,
            z: z as i32,
            dimension: dimension as u32,
        }
    }

    pub fn chunk_indexes_near_chunk_index(ctx: &ReducerContext, chunk_index: u64, radius: i32) -> Vec<u64> {
        let chunk_coord = Self::chunk_coord_from_chunk_index(chunk_index);
        Self::chunk_indexes_near(ctx, chunk_coord.x, chunk_coord.z, chunk_coord.dimension, radius)
    }

    pub fn chunk_indexes_near(ctx: &ReducerContext, chunk_x: i32, chunk_z: i32, dimension: u32, radius: i32) -> Vec<u64> {
        if dimension != dimensions::OVERWORLD {
            panic!("Only overworld dimension is currently supported");
        }

        let world_info = ctx.db.world_region_state().id().find(0).unwrap();

        let min_x = (chunk_x - radius).max(world_info.region_min_chunk_x as i32);
        let min_z = (chunk_z - radius).max(world_info.region_min_chunk_z as i32);
        let max_x = (chunk_x + radius)
            .min((world_info.region_min_chunk_x + world_info.region_width_chunks * world_info.region_count_sqrt as u16) as i32 - 1);
        let max_z = (chunk_z + radius)
            .min((world_info.region_min_chunk_z + world_info.region_height_chunks * world_info.region_count_sqrt as u16) as i32 - 1);

        let mut chunk_indexes = Vec::new();

        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinatesMessage {
                    x,
                    z,
                    dimension: dimensions::OVERWORLD,
                });
                chunk_indexes.push(chunk_index);
            }
        }
        chunk_indexes
    }
}
