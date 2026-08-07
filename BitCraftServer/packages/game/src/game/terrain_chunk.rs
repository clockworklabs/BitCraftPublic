use spacetimedb::{ReducerContext, Table};

use super::coordinates::ChunkCoordinates;
use super::dimensions;
use crate::game::coordinates::*;
use crate::messages::components::{TerrainCell, TerrainChunkState};
use crate::messages::game_util::DimensionType;
use crate::messages::util::ChunkCoordinatesMessage;
use crate::{dimension_description_state, terrain_chunk_state};
use std::collections::BTreeMap;

/// A cache to put all fetched `TerrainChunkState`s in,
/// so that we avoid going over the WASM boundary for these large objects.
pub struct TerrainChunkCache(BTreeMap<u64, Option<TerrainChunkState>>);

impl TerrainChunkCache {
    /// Returns a cache with no loaded entries.
    pub fn empty() -> Self {
        Self(<_>::default())
    }

    /// Prime the cache with all entries available.
    pub fn fetch(ctx: &ReducerContext) -> Self {
        Self(ctx.db.terrain_chunk_state().iter().map(|s| (s.chunk_index, Some(s))).collect())
    }

    /// Iterate over all the terrain chunks loaded thus far.
    pub fn iter_cached_values(&self) -> impl Iterator<Item = &TerrainChunkState> {
        self.0.values().flatten()
    }

    /// Persist whatever changes where made to `index` if it was in the cache.
    pub fn persist(&self, ctx: &ReducerContext, index: u64) -> bool {
        let Some(Some(state)) = self.0.get(&index) else {
            return false;
        };

        ctx.db.terrain_chunk_state().chunk_index().update(state.clone());
        true
    }

    /// Persist whatever changes where made to `index` if it was in the cache.
    ///
    /// Unlike `persist`, this method consumes the entire cache.
    pub fn consume_and_persist(mut self, ctx: &ReducerContext, index: u64) -> bool {
        let Some(Some(state)) = self.0.remove(&index) else {
            return false;
        };

        ctx.db.terrain_chunk_state().chunk_index().update(state.clone());
        true
    }

    /// Persist whatever changes where made to all cached chunks
    pub fn consume_and_persist_all(self, ctx: &ReducerContext) {
        for (_, item) in self.0.iter() {
            if let Some(item) = item {
                ctx.db.terrain_chunk_state().chunk_index().update(item.clone());
            }
        }
    }

    /// Retrieves the state corresponding to the index.
    /// If it is not available in the cache, it will first be loaded into it.
    ///
    /// NOTE: This provides mutable access to the [`TerrainChunkState`] in the cache.
    /// Be sure to persist these changes via `cache.persist(index)` if needed.
    pub fn filter_by_chunk_index(&mut self, ctx: &ReducerContext, index: u64) -> Option<&mut TerrainChunkState> {
        self.0
            .entry(index)
            .or_insert_with(|| ctx.db.terrain_chunk_state().chunk_index().find(&index))
            .as_mut()
    }

    /// Retrieves the state corresponding to the given `chunk_coords`.
    /// If it is not available in the cache, it will first be loaded into it.
    ///
    /// NOTE: This provides mutable access to the [`TerrainChunkState`] in the cache.
    /// Be sure to persist these changes via `cache.persist(index)` if needed.
    pub fn get_from_chunk_coordinates(&mut self, ctx: &ReducerContext, chunk_coords: ChunkCoordinates) -> Option<&mut TerrainChunkState> {
        self.filter_by_chunk_index(ctx, TerrainChunkState::chunk_index_from_coords(&chunk_coords))
    }

    /// Retrieves a [`TerrainCell`] corresponding to the given `coords`,
    /// deriving it from some [`TerrainChunkState`].
    /// If this state is not available in the cache, it will first be loaded into it.
    pub fn get_terrain_cell(&mut self, ctx: &ReducerContext, coords: &LargeHexTile) -> Option<TerrainCell> {
        let chunk = self.get_from_chunk_coordinates(ctx, ChunkCoordinates::from(coords))?;
        chunk.get_entity(&coords.to_offset_coordinates())
    }
}

impl TerrainChunkState {
    pub const WIDTH: u32 = 32;
    pub const HEIGHT: u32 = 32;

    const BUFFER_LENGTH: usize = (TerrainChunkState::WIDTH * TerrainChunkState::HEIGHT) as usize;

    pub fn default_with_capacity() -> Self {
        Self {
            chunk_index: 0,

            chunk_x: 0,
            chunk_z: 0,
            dimension: DimensionType::Overworld as u32,
            biomes: vec![0; TerrainChunkState::BUFFER_LENGTH],
            elevations: vec![0; TerrainChunkState::BUFFER_LENGTH],
            water_levels: vec![0; TerrainChunkState::BUFFER_LENGTH],
            water_body_types: vec![0; TerrainChunkState::BUFFER_LENGTH],
            zoning_types: vec![0; TerrainChunkState::BUFFER_LENGTH],
            original_elevations: vec![0; TerrainChunkState::BUFFER_LENGTH],
            biome_density: vec![0; TerrainChunkState::BUFFER_LENGTH],
        }
    }

    pub fn chunk_index_from_coords(coords: &ChunkCoordinates) -> u64 {
        return coords.chunk_index();
    }

    pub fn get_entity(&self, offset: &OffsetCoordinatesLarge) -> Option<TerrainCell> {
        let OffsetCoordinatesLarge { x, z, dimension } = offset;
        let local_x = x % TerrainChunkState::WIDTH as i32;
        let local_z = z % TerrainChunkState::HEIGHT as i32;
        self.try_get_entity_local(local_x, local_z, *dimension)
    }

    pub fn get_entity_static(ctx: &ReducerContext, coord: &LargeHexTile) -> Option<TerrainCell> {
        let index = Self::chunk_index_from_coords(&coord.chunk_coordinates());
        if let Some(chunk) = ctx.db.terrain_chunk_state().chunk_index().find(&index) {
            return chunk.get_entity(&coord.to_offset_coordinates());
        }
        None
    }

    pub fn try_get_entity_local(&self, local_x: i32, local_z: i32, dimension: u32) -> Option<TerrainCell> {
        if local_x < 0
            || local_z < 0
            || local_x >= TerrainChunkState::WIDTH as i32
            || local_z >= TerrainChunkState::HEIGHT as i32
        {
            return None;
        }

        Some(self.get_entity_local(local_x, local_z, dimension))
    }

    pub fn get_entity_local(&self, local_x: i32, local_z: i32, dimension: u32) -> TerrainCell {
        let index = (local_z * TerrainChunkState::WIDTH as i32 + local_x) as u32;

        TerrainCell {
            x: (self.chunk_x * (TerrainChunkState::WIDTH as i32)) + local_x,
            z: (self.chunk_z * (TerrainChunkState::HEIGHT as i32)) + local_z,
            biomes: self.biomes[index as usize],
            elevation: self.elevations[index as usize],
            water_level: self.water_levels[index as usize],
            water_body_type: self.water_body_types[index as usize],
            zoning_type: self.zoning_types[index as usize],
            original_elevation: self.original_elevations[index as usize],
            dimension,
            biome_density: self.biome_density[index as usize],
        }
    }

    pub fn set_entity(&mut self, offset: OffsetCoordinatesLarge, entity: TerrainCell) {
        let OffsetCoordinatesLarge { x, z, dimension: _ } = offset;
        let local_x = x % TerrainChunkState::WIDTH as i32;
        let local_z = z % TerrainChunkState::HEIGHT as i32;
        let index = (local_z * TerrainChunkState::WIDTH as i32 + local_x) as u32;

        self.biomes[index as usize] = entity.biomes;
        self.elevations[index as usize] = entity.elevation;
        self.water_levels[index as usize] = entity.water_level;
        self.zoning_types[index as usize] = entity.zoning_type;
        self.water_body_types[index as usize] = entity.water_body_type;
        self.original_elevations[index as usize] = entity.original_elevation;
        self.biome_density[index as usize] = entity.biome_density;
    }

    pub fn local_iter(&self, dimension: u32) -> TerrainChunkIter<'_> {
        TerrainChunkIter {
            chunk: self,
            x: 0,
            z: 0,
            dimension,
        }
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

    pub fn coordinates(&self) -> ChunkCoordinates {
        return ChunkCoordinates {
            x: self.chunk_x,
            z: self.chunk_z,
            dimension: self.dimension,
        };
    }

    pub fn chunk_indexes_near_chunk_index(ctx: &ReducerContext, chunk_index: u64, radius: i32) -> Vec<u64> {
        let chunk_coord = Self::chunk_coord_from_chunk_index(chunk_index);
        Self::chunk_indexes_near(ctx, chunk_coord.x, chunk_coord.z, chunk_coord.dimension, radius)
    }

    pub fn chunk_indexes_near(ctx: &ReducerContext, chunk_x: i32, chunk_z: i32, dimension: u32, radius: i32) -> Vec<u64> {
        let dimension_state = ctx.db.dimension_description_state().dimension_id().find(&dimension).unwrap();

        let min_x = (chunk_x - radius).max(dimension_state.dimension_position_large_x as i32);
        let min_z = (chunk_z - radius).max(dimension_state.dimension_position_large_z as i32);
        let max_x =
            (chunk_x + radius).min(dimension_state.dimension_position_large_x as i32 + dimension_state.dimension_size_large_x as i32 - 1);
        let max_z =
            (chunk_z + radius).min(dimension_state.dimension_position_large_z as i32 + dimension_state.dimension_size_large_z as i32 - 1);

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

    pub fn get_index(&self, coord: LargeHexTile) -> usize {
        let offset = coord.to_offset_coordinates();
        let local_x = offset.x % TerrainChunkState::WIDTH as i32;
        let local_z = offset.z % TerrainChunkState::HEIGHT as i32;
        let index = (local_z * TerrainChunkState::WIDTH as i32 + local_x) as usize;
        return index;
    }

    pub fn get_water_body_type(&self, coord: LargeHexTile) -> Option<u8> {
        let index = self.get_index(coord);
        return self.get_water_body_type_index(index);
    }

    pub fn get_water_body_type_index(&self, index: usize) -> Option<u8> {
        if !self.is_submerged_index(index) {
            return None;
        }
        return Some(self.water_body_types[index]);
    }

    pub fn is_submerged_index(&self, index: usize) -> bool {
        return self.water_levels[index] > self.elevations[index];
    }

    pub fn get_water_depth_index(&self, index: usize) -> i16 {
        return (self.water_levels[index] - self.elevations[index]).max(0);
    }

    pub fn get_biome_index(&self, index: usize) -> u32 {
        return self.biomes[index];
    }
}

pub struct TerrainChunkIter<'a> {
    chunk: &'a TerrainChunkState,
    x: u32,
    z: u32,
    dimension: u32,
}

impl<'a> Iterator for TerrainChunkIter<'a> {
    type Item = TerrainCell;

    fn next(&mut self) -> Option<Self::Item> {
        if self.z >= TerrainChunkState::HEIGHT {
            return None;
        }

        let result = self.chunk.get_entity_local(self.x as i32, self.z as i32, self.dimension);
        self.x += 1;
        if self.x >= TerrainChunkState::WIDTH {
            self.x = 0;
            self.z += 1;
        }

        Some(result)
    }
}
