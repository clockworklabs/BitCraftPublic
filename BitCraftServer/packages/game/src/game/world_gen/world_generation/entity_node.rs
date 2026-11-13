use super::{
    super::super::{world_gen::building_details::BuildingDetails, world_gen::resource_definition::ResourceDetails},
    hex_graph::{HexGraph, HexNode},
    terrain_node::TerrainNode,
};
use crate::game::{coordinates::hex_coordinates::HexCoordinates, unity_helpers::vector2::Vector2};

#[derive(Default, Clone)]
pub struct ResourceData {
    pub details: Option<ResourceDetails>,
    pub resource_id: i32,
    pub direction: i32,
}

#[derive(Default, Clone)]
pub struct EntityNode {
    pub coordinates: HexCoordinates,
    pub building: Option<BuildingDetails>,
    pub resource: Option<ResourceData>,
}

impl HexNode for EntityNode {
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

impl EntityNode {
    pub fn world_position(&self) -> Vector2 {
        self.coordinates.to_center_position_xz(false)
    }

    pub fn get_elevation(&self, terrain_graph: &HexGraph<TerrainNode>) -> i16 {
        let terrain_coordinates = self.coordinates.get_terrain_coordinates();
        let terrain_indices = [
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[0]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[1]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[2]),
        ];

        let terrain_nodes = [
            terrain_graph.get(terrain_indices[0]),
            terrain_graph.get(terrain_indices[1]),
            terrain_graph.get(terrain_indices[2]),
        ];

        // let terrain_nodes = self.get_terrain_nodes(terrain_graph);

        let elevations = [
            if terrain_nodes[0].is_some() {
                terrain_nodes[0].unwrap().elevation
            } else {
                -1
            },
            if terrain_nodes[1].is_some() {
                terrain_nodes[1].unwrap().elevation
            } else {
                -1
            },
            if terrain_nodes[2].is_some() {
                terrain_nodes[2].unwrap().elevation
            } else {
                -1
            },
        ];

        // invalid location
        if elevations[0] == -1 || elevations[1] == -1 || elevations[2] == -1 {
            return -1;
        }

        elevations[0]
    }

    pub fn min_max_elevation(&self, terrain_graph: &HexGraph<TerrainNode>) -> Vec<i16> {
        let terrain_coordinates = self.coordinates.get_terrain_coordinates();
        let terrain_indices = [
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[0]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[1]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[2]),
        ];

        let terrain_nodes = [
            terrain_graph.get(terrain_indices[0]),
            terrain_graph.get(terrain_indices[1]),
            terrain_graph.get(terrain_indices[2]),
        ];

        // let terrain_nodes = self.get_terrain_nodes(terrain_graph);

        let elevations = [
            if terrain_nodes[0].is_some() {
                terrain_nodes[0].unwrap().elevation
            } else {
                -1
            },
            if terrain_nodes[1].is_some() {
                terrain_nodes[1].unwrap().elevation
            } else {
                -1
            },
            if terrain_nodes[2].is_some() {
                terrain_nodes[2].unwrap().elevation
            } else {
                -1
            },
        ];

        let min = *elevations.iter().min().unwrap();
        let max = *elevations.iter().max().unwrap();
        vec![min, max]
    }

    pub fn get_biome_value(&self, biome_index: usize, terrain_graph: &HexGraph<TerrainNode>) -> f32 {
        let terrain_coordinates = self.coordinates.get_terrain_coordinates()[0];
        let terrain_index = terrain_graph.get_index_from_hex_coordinates(terrain_coordinates);
        let terrain_node = terrain_graph.get(terrain_index);

        // let terrain_node = self.get_terrain_nodes(terrain_graph)[0];

        if terrain_node.is_none() {
            return 0f32;
        }
        let terrain_node = terrain_node.unwrap();

        let biome_multiplier = terrain_node.biomes_multipliers[biome_index];
        if biome_multiplier <= 0f32 {
            return 0f32;
        }

        let mut total_biomes_multipliers = 0f32;
        for i in 0..terrain_node.biomes_multipliers.len() {
            let mult = terrain_node.biomes_multipliers[i];
            if mult < 0f32 {
                continue;
            }
            total_biomes_multipliers += mult;
        }

        biome_multiplier / total_biomes_multipliers
    }

    pub fn is_underwater(&self, terrain_graph: &HexGraph<TerrainNode>) -> bool {
        let terrain_coordinates = self.coordinates.get_terrain_coordinates();
        let terrain_indices = [
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[0]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[1]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[2]),
        ];

        let terrain_nodes = [
            terrain_graph.get(terrain_indices[0]),
            terrain_graph.get(terrain_indices[1]),
            terrain_graph.get(terrain_indices[2]),
        ];

        // let terrain_nodes = self.get_terrain_nodes(terrain_graph);

        if terrain_nodes[0].is_some() && terrain_nodes[0].unwrap().is_water() {
            return true;
        }
        if terrain_nodes[1].is_some() && terrain_nodes[1].unwrap().is_water() {
            return true;
        }
        if terrain_nodes[2].is_some() && terrain_nodes[2].unwrap().is_water() {
            return true;
        }

        false
    }

    pub fn get_water_level(&self, terrain_graph: &HexGraph<TerrainNode>) -> i16 {
        let terrain_coordinates = self.coordinates.get_terrain_coordinates();
        let terrain_indices = [
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[0]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[1]),
            terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[2]),
        ];

        let terrain_nodes = [
            terrain_graph.get(terrain_indices[0]),
            terrain_graph.get(terrain_indices[1]),
            terrain_graph.get(terrain_indices[2]),
        ];

        let mut water_level = i16::MAX;
        for node in terrain_nodes.into_iter().flatten() {
            if node.water_level < water_level {
                water_level = node.water_level;
            }
        }

        water_level
    }

    pub fn get_water_depth(&self, terrain_graph: &HexGraph<TerrainNode>) -> i16 {
        self.get_water_level(terrain_graph) - self.get_elevation(terrain_graph)
    }

    pub fn is_valid_elevation(&self, terrain_graph: &HexGraph<TerrainNode>, resource_details: &ResourceDetails) -> bool {
        let is_underwater = self.is_underwater(terrain_graph);
        if resource_details.spawns_on_land && !is_underwater {
            let elevation = self.get_elevation(terrain_graph);
            let water_level = self.get_water_level(terrain_graph);
            if i32::from(elevation - water_level) >= resource_details.land_elevation_range.x
                && i32::from(elevation - water_level) <= resource_details.land_elevation_range.y
            {
                return true;
            }
        }

        if resource_details.spawns_in_water && is_underwater {
            let water_depth = self.get_water_depth(terrain_graph);
            if i32::from(water_depth) >= resource_details.water_depth_range.x
                && i32::from(water_depth) <= resource_details.water_depth_range.y
            {
                return true;
            }
        }

        false
    }

    // fn get_terrain_nodes(&self, terrain_graph: &HexGraph<TerrainNode>) -> [Option<&TerrainNode>; 3] {
    //     let terrain_coordinates = self.coordinates.get_terrain_coordinates();
    //     let terrain_indices = [
    //         terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[0]),
    //         terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[1]),
    //         terrain_graph.get_index_from_hex_coordinates(terrain_coordinates[2]),
    //     ];

    //     return [
    //         terrain_graph.get(terrain_indices[0]),
    //         terrain_graph.get(terrain_indices[1]),
    //         terrain_graph.get(terrain_indices[2]),
    //     ];
    // }
}
