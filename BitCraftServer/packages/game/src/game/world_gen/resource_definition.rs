use super::noise_helper;
use super::noise_specs::NoiseSpecs;
use crate::game::unity_helpers::vector2::Vector2;
use crate::game::unity_helpers::vector2int::Vector2Int;

#[derive(Debug)]
pub struct ResourceDefinition {
    pub resource_details: ResourceDetails,
    pub biomes: Vec<ResourceBiome>,
}

impl ResourceDefinition {
    pub fn count(&self) -> i32 {
        self.biomes.len() as i32
    }

    pub fn get_biome(&self, index: i32) -> Option<&ResourceBiome> {
        if self.biomes.is_empty() {
            return None;
        }
        let index = index.clamp(0, self.biomes.len() as i32 - 1);
        Some(&self.biomes[index as usize])
    }

    pub fn get_biome_mut(&mut self, index: i32) -> Option<&mut ResourceBiome> {
        if self.biomes.is_empty() {
            return None;
        }
        let index = index.clamp(0, self.biomes.len() as i32 - 1);
        Some(&mut self.biomes[index as usize])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ResourceDetails {
    pub clump_id: i32,
    pub spawns_on_land: bool,
    pub land_elevation_range: Vector2Int,
    pub spawns_in_water: bool,
    pub water_depth_range: Vector2Int,
    pub max_elevation_delta: i32,
}

#[derive(Debug)]
pub struct ResourceBiome {
    pub biome_index: i32,
    pub chance: f32,              //[0..1]
    pub noise_threshold: Vector2, //[0..1]
    pub noise_specs: NoiseSpecs,
}

impl ResourceBiome {
    pub fn get(&self, x: f32, y: f32) -> f32 {
        noise_helper::get(
            Vector2 { x, y },
            self.noise_specs.scale,
            self.noise_specs.octaves,
            self.noise_specs.persistance,
            self.noise_specs.lacunarity,
            self.noise_specs.offset,
        )
    }
}
