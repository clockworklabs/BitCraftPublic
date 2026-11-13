use super::super::unity_helpers::animation_curve::AnimationCurve;
use super::noise_based_elevation_layer::{BlendingMode, NoiseBasedElevationLayer};
use super::noise_helper;
use super::noise_specs::NoiseSpecs;
use super::river_generation_settings::RiverGenerationSettings;
use crate::game::unity_helpers::float_helper::f32::{inverse_lerp, map};
use crate::game::unity_helpers::vector2::Vector2;

#[derive(Debug)]
pub struct BiomeDefinition {
    pub distance_to_sea_curve: AnimationCurve,
    pub distance_to_biomes_curve: AnimationCurve,
    pub transition_length: i32,
    pub noise_sea_multiplier: AnimationCurve,
    pub noise_based_elevation_layers: Vec<NoiseBasedElevationLayer>,
    pub max_lake_depth: i16,
    pub terracing: bool,
    pub grass_density: i16,
    pub lake_noise_specs: NoiseSpecs,
    pub lake_noise_threshold: f32,
    pub lake_depth_multiplier: i16,
    pub lake_depth_smoothing: f32,
    pub lake_sea_barriers: bool,
    pub river_generation_settings: Option<RiverGenerationSettings>,
}

impl BiomeDefinition {
    pub fn get_elevation_from_noise_layers(&self, x: f32, y: f32) -> f32 {
        let mut total_elevation = 0f32;
        let mut divisor = 0;

        for i in 0..self.noise_based_elevation_layers.len() {
            let layer = &self.noise_based_elevation_layers[i];
            let noise = noise_helper::get(
                Vector2 { x, y },
                layer.noise.scale,
                layer.noise.octaves,
                layer.noise.persistance,
                layer.noise.lacunarity,
                layer.noise.offset,
            );

            if i > 0 && noise < layer.threshold {
                continue;
            }

            let normalized_noise = inverse_lerp(layer.threshold, 1f32, noise);
            let elevation = map(normalized_noise, 0f32, 1f32, layer.range.x as f32, layer.range.y as f32);

            match layer.blending_mode {
                BlendingMode::Add => {
                    total_elevation += elevation;
                    divisor += 1;
                }
                BlendingMode::Override => {
                    total_elevation = elevation;
                    divisor = 1;
                }
            }
        }

        total_elevation / divisor as f32
    }

    pub fn get_lake_noise(&self, x: f32, y: f32) -> f32 {
        noise_helper::get(
            Vector2 { x, y },
            self.lake_noise_specs.scale,
            self.lake_noise_specs.octaves,
            self.lake_noise_specs.persistance,
            self.lake_noise_specs.lacunarity,
            self.lake_noise_specs.offset,
        )
    }
}
