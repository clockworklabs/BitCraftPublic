use spacetimedb::ReducerContext;

use super::hex_coordinates::HexCoordinates;
use super::offset_coordinates::OffsetCoordinates;

use crate::game::coordinates::*;
use crate::messages::components::TerrainChunkState;
use crate::messages::generic::world_region_state;
use std::convert::From;
use std::fmt::Display;

impl ChunkCoordinates {
    pub fn surrounding_and_including(&self, ctx: &ReducerContext) -> Vec<ChunkCoordinates> {
        let globals = ctx.db.world_region_state().id().find(0).unwrap();

        let mut chunks = vec![*self];
        let (x, z) = (self.x, self.z);
        let side = 3;
        let hside = (side - 1) / 2;
        let max_x = if globals.region_index == 0 {
            globals.region_min_chunk_x + globals.region_width_chunks * globals.region_count_sqrt as u16
        } else {
            globals.region_min_chunk_x + globals.region_width_chunks
        } as i32;
        let max_z = if globals.region_index == 0 {
            globals.region_min_chunk_z + globals.region_height_chunks * globals.region_count_sqrt as u16
        } else {
            globals.region_min_chunk_z + globals.region_height_chunks
        } as i32;
        for i in (x - hside)..=(x + hside) {
            for j in (z - hside)..=(z + hside) {
                if i == x && j == z {
                    continue;
                }
                if i < globals.region_min_chunk_x as i32 || j < globals.region_min_chunk_z as i32 || i >= max_x || j >= max_z {
                    continue;
                }
                chunks.push(ChunkCoordinates {
                    x: i,
                    z: j,
                    dimension: self.dimension,
                });
            }
        }
        chunks
    }

    pub fn from_hashcode(hashcode: i64) -> Self {
        let offset = OffsetCoordinates::from_hashcode(hashcode);
        ChunkCoordinates {
            x: offset.x,
            z: offset.z,
            dimension: offset.dimension,
        }
    }

    pub fn hashcode(&self) -> i64 {
        OffsetCoordinates {
            x: self.x,
            z: self.z,
            dimension: self.dimension,
        }
        .hashcode()
    }

    pub fn from_terrain_coordinates(coords: LargeHexTile) -> Self {
        let offset = coords.to_offset_coordinates();
        ChunkCoordinates {
            x: offset.x / (TerrainChunkState::WIDTH as i32),
            z: offset.z / (TerrainChunkState::HEIGHT as i32),
            dimension: coords.dimension,
        }
    }

    pub fn from_coordinates(coords: SmallHexTile) -> Self {
        let offset = coords.to_offset_coordinates();
        ChunkCoordinates {
            x: offset.x / ((3 * TerrainChunkState::WIDTH) as i32),
            z: offset.z / ((3 * TerrainChunkState::HEIGHT) as i32),
            dimension: coords.dimension,
        }
    }

    pub fn chunk_index(self) -> u64 {
        if self.x < 0 || self.z < 0 || self.dimension == 0 {
            return 0;
        }

        (self.dimension as u64 - 1) * 1000000 + self.z as u64 * 1000 + self.x as u64 + 1
        // 1000 is over the maximum chunk size and will skip a table access at runtime
    }
}

impl PartialOrd for ChunkCoordinates {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChunkCoordinates {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hashcode().cmp(&other.hashcode())
    }
}

impl From<&HexCoordinates> for ChunkCoordinates {
    fn from(coordinates: &HexCoordinates) -> Self {
        ChunkCoordinates::from(OffsetCoordinates::from(coordinates))
    }
}

impl From<HexCoordinates> for ChunkCoordinates {
    fn from(coordinates: HexCoordinates) -> Self {
        ChunkCoordinates::from(OffsetCoordinates::from(coordinates))
    }
}

impl From<&OffsetCoordinates> for ChunkCoordinates {
    fn from(offset: &OffsetCoordinates) -> Self {
        let terrain_offset = offset.scale(1.0 / 3.0);
        if terrain_offset.x < 0 || terrain_offset.z < 0 {
            // A negative value above -TerrainChunkState::WIDTH will return a x or z of 0, but the matching terrain tile does not exit in that chunk
            return ChunkCoordinates {
                x: -1,
                z: -1,
                dimension: offset.dimension,
            };
        }
        ChunkCoordinates {
            x: terrain_offset.x / TerrainChunkState::WIDTH as i32,
            z: terrain_offset.z / TerrainChunkState::HEIGHT as i32,
            dimension: offset.dimension,
        }
    }
}

impl From<OffsetCoordinates> for ChunkCoordinates {
    fn from(offset: OffsetCoordinates) -> Self {
        ChunkCoordinates::from(&offset)
    }
}

impl From<&OffsetCoordinatesLarge> for ChunkCoordinates {
    fn from(offset: &OffsetCoordinatesLarge) -> Self {
        ChunkCoordinates {
            x: offset.x / TerrainChunkState::WIDTH as i32,
            z: offset.z / TerrainChunkState::HEIGHT as i32,
            dimension: offset.dimension,
        }
    }
}

impl From<OffsetCoordinatesLarge> for ChunkCoordinates {
    fn from(offset: OffsetCoordinatesLarge) -> Self {
        ChunkCoordinates::from(&offset)
    }
}

impl Display for ChunkCoordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChunkCoordinates ({}, {}, {})", self.x, self.z, self.dimension)
    }
}
