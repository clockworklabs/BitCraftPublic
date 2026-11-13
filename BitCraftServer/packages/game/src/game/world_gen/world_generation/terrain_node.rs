use super::hex_graph::HexNode;
use crate::game::coordinates::hex_coordinates::HexCoordinates;
use crate::game::unity_helpers::vector2::Vector2;

#[derive(Default, Clone)]
pub struct TerrainNode {
    pub coordinates: HexCoordinates,
    pub node_type: NodeType,
    pub distance_to_sea: i32,
    pub distance_to_sea_relative: f32,
    pub distance_to_water: i32,
    pub distance_to_water_relative: f32,
    pub biomes: u32,
    pub distances_to_biomes: Vec<i32>,
    pub biomes_multipliers: Vec<f32>,
    pub distance_to_different_biomes: i32,
    pub distance_to_different_biomes_relative: f32,
    pub elevation: i16,
    pub water_level: i16,
    pub biome_density: u32,
    pub lake_depth: i16,
}

impl HexNode for TerrainNode {
    fn new(x: i32, z: i32) -> Self {
        Self {
            coordinates: HexCoordinates::from_offset_coordinates(x, z, 1),
            ..Default::default()
        }
    }

    fn get_coordinates(&self) -> HexCoordinates {
        self.coordinates
    }
}

#[derive(PartialEq, Clone)]
#[derive(Default)]
pub enum NodeType {
    #[default]
    Sea,
    Lake,
    Land,
    River,
}


impl TerrainNode {
    pub fn world_position(&self) -> Vector2 {
        self.coordinates.to_center_position_xz(true)
    }

    pub fn is_water(&self) -> bool {
        self.node_type == NodeType::Sea || self.node_type == NodeType::Lake || self.node_type == NodeType::River
    }

    pub fn biome(&self) -> u32 {
        self.biomes & 0xFF
    }
}
