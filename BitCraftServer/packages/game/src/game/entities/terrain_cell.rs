pub use crate::game::coordinates::*;
pub use crate::messages::components::TerrainCell;
use crate::{game::PLAYER_MIN_SWIM_DEPTH, Biome};

impl TerrainCell {
    pub fn is_submerged(&self) -> bool {
        self.water_level > self.elevation
    }

    pub fn coordinates(&self) -> LargeHexTile {
        LargeHexTile::from(OffsetCoordinatesLarge {
            x: self.x,
            z: self.z,
            dimension: self.dimension,
        })
    }

    pub fn biome(&self) -> i32 {
        (self.biomes & 0xFF) as i32
    }

    pub fn biome_percentage(&self, biome: Biome) -> f32 {
        //Inverses the encoding that's done during world-gen
        for i in 0..4 {
            let biome_index = (self.biomes >> (i * 8)) & 0xFF;
            if biome_index != biome as u32 {
                continue;
            }

            let biome_percentage = (self.biome_density >> (i * 8)) & 0xFF;
            return biome_percentage as f32 / 128f32;
        }

        0f32
    }

    pub fn surface_level(&self) -> i16 {
        self.elevation.max(self.water_level)
    }

    pub fn water_depth(&self) -> i16 {
        (self.water_level - self.elevation).max(0)
    }

    /// Returns elevation that player should walk/swim on. Takes care of all elevation vs water level checks
    pub fn player_surface_elevation(&self) -> i16 {
        let elevation = self.elevation;
        let water_level = self.water_level;
        let water_depth = water_level - elevation;
        if water_depth >= PLAYER_MIN_SWIM_DEPTH {
            return water_level;
        }
        elevation
    }

    pub fn player_should_swim(&self) -> bool {
        self.water_depth() >= PLAYER_MIN_SWIM_DEPTH
    }
}
