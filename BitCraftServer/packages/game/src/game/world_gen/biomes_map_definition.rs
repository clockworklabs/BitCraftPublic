use super::super::unity_helpers::{vector2::Vector2, vector2int::Vector2Int};
use super::biome_definition::BiomeDefinition;
use crate::game::unity_helpers::float_helper::f32::half_to_even;

pub const SPAWN_MASK: u8 = 1 << 7;
pub const MAX_BIOMES: u8 = SPAWN_MASK - 1;
pub const STEP: i32 = super::world_definition::STEP * 6;

#[derive(Debug)]
pub struct BiomesMapDefinition {
    pub biomes: Vec<BiomeDefinition>,
    pub values: Vec<u8>,
}

impl BiomesMapDefinition {
    pub fn count(&self) -> i32 {
        self.biomes.len() as i32
    }

    pub fn get_biome(&self, index: i32) -> Option<&BiomeDefinition> {
        if self.biomes.is_empty() {
            return None;
        }
        let index = index.clamp(0, self.biomes.len() as i32 - 1);
        let biome = &self.biomes[index as usize];
        biome.into()
    }

    fn get_pixel_coordinates(world_position: Vector2) -> Vector2Int {
        Vector2Int {
            x: half_to_even(world_position.x / STEP as f32) as i32,
            y: half_to_even(world_position.y / STEP as f32) as i32,
        }
    }

    fn get_pixel_index(&self, coordinates: Vector2Int) -> Option<usize> {
        let x = coordinates.x;
        let y = coordinates.y;
        if x < 0 || y < 0 {
            return None;
        }

        if x >= y {
            return Some((x * x + y) as usize);
        }

        Some((y * y + 2 * y + 2 - x) as usize)
    }

    pub fn get_index_at_pos(&self, world_position: Vector2) -> i32 {
        self.get_index_at_coord(Self::get_pixel_coordinates(world_position))
    }

    fn get_index_at_coord(&self, coordinates: Vector2Int) -> i32 {
        self.get_index_at(self.get_pixel_index(coordinates))
    }

    fn get_index_at(&self, index: Option<usize>) -> i32 {
        if index.is_none() {
            return 0;
        }
        let index = index.unwrap();
        if index >= self.values.len() {
            return 0;
        }

        let biome_index = self.values[index] & !SPAWN_MASK;
        biome_index as i32
    }

    pub fn is_spawn_at_pos(&self, world_position: Vector2) -> bool {
        self.is_spawn_at_coord(Self::get_pixel_coordinates(world_position))
    }

    fn is_spawn_at_coord(&self, coordinates: Vector2Int) -> bool {
        self.is_spawn_at(self.get_pixel_index(coordinates))
    }

    fn is_spawn_at(&self, index: Option<usize>) -> bool {
        if index.is_none() {
            return false;
        }
        let index = index.unwrap();
        if index >= self.values.len() {
            return false;
        }

        (self.values[index] & SPAWN_MASK) > 0
    }
}
