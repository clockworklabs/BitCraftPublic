use spacetimedb::{ReducerContext, Table};

use crate::{messages::generic::world_region_state, utils::from_ctx::FromCtx};

use super::*;

#[derive(Debug, Copy, Clone)]
pub struct RegionCoordinates {
    pub x: u8,
    pub z: u8,
}

impl RegionCoordinates {
    //Indices start at 1 (0 is reserved for global module)
    pub fn from_region_index(region_index: u8, region_count_sqrt: u8) -> Self {
        Self {
            x: (region_index - 1) % region_count_sqrt,
            z: (region_index - 1) / region_count_sqrt,
        }
    }

    pub fn to_region_index(&self, region_count_sqrt: u8) -> u8 {
        self.z * region_count_sqrt + self.x + 1
    }
}

impl FromCtx<FloatHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: FloatHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<&FloatHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: &FloatHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<SmallHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: SmallHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<&SmallHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: &SmallHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<LargeHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: LargeHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<&LargeHexTile> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: &LargeHexTile) -> Self {
        Self::from_ctx(ctx, value.chunk_coordinates())
    }
}

impl FromCtx<ChunkCoordinates> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: ChunkCoordinates) -> Self {
        let region = ctx.db.world_region_state().iter().next().unwrap();
        let x = value.x as u16 / region.region_width_chunks;
        let z = value.z as u16 / region.region_width_chunks;
        RegionCoordinates { x: x as u8, z: z as u8 }
    }
}

impl FromCtx<&ChunkCoordinates> for RegionCoordinates {
    fn from_ctx(ctx: &ReducerContext, value: &ChunkCoordinates) -> Self {
        Self::from_ctx(ctx, *value)
    }
}
