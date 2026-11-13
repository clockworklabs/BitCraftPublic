use spacetimedb::log;

use super::super::unity_helpers::animation_curve::AnimationCurve;
use crate::messages::world_gen::{WorldGenRiverGenerationSettings, WorldGenRiverPathfindingCosts};

#[derive(Debug)]
pub struct RiverGenerationSettings {
    pub radius: i32,
    pub depth_curve: AnimationCurve,
    pub erosion: f32,
    pub min_lake_circumference: i32,
    pub pathfinding_node_limit: i32,
    pub pathfinding_costs: Vec<RiverPathfindingCosts>,
}

impl RiverGenerationSettings {
    pub fn new(river_generation_settings: WorldGenRiverGenerationSettings) -> Self {
        Self {
            radius: river_generation_settings.radius,
            depth_curve: river_generation_settings.depth_curve,
            erosion: river_generation_settings.erosion,
            min_lake_circumference: river_generation_settings.min_lake_circumference,
            pathfinding_node_limit: river_generation_settings.pathfinding_node_limit,
            pathfinding_costs: river_generation_settings.pathfinding_costs,
        }
    }

    pub fn get_pathfinding_costs(&self, elevation_difference: i32) -> f32 {
        for pathfinding_cost in self.pathfinding_costs.iter() {
            if elevation_difference >= pathfinding_cost.elevation_difference_range.x
                && elevation_difference <= pathfinding_cost.elevation_difference_range.y
            {
                return pathfinding_cost.pathfinding_costs;
            }
        }

        log::error!(
            "RiverGenerationSettings doesn't have have a pathfinding-cost for an elevation difference of {}",
            elevation_difference
        );
        0f32
    }
}

pub type RiverPathfindingCosts = WorldGenRiverPathfindingCosts;
