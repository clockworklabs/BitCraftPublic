use super::super::unity_helpers::vector2::Vector2;
use crate::{game::unity_helpers::common_rng::CommonRNG, messages::world_gen::WorldGenNoiseSpecs};

#[derive(Debug)]
pub struct NoiseSpecs {
    pub seed: i32,
    pub scale: f32,
    pub octaves: i32,
    pub persistance: f32,
    pub lacunarity: f32,
    pub offset: Vector2,
}

impl NoiseSpecs {
    pub fn new(noise_specs: &WorldGenNoiseSpecs, offset_min_max_value: f32) -> Self {
        Self {
            seed: noise_specs.seed,
            scale: noise_specs.scale,
            octaves: noise_specs.octaves,
            persistance: noise_specs.persistance,
            lacunarity: noise_specs.lacunarity,
            offset: Self::get_offset(noise_specs.seed, offset_min_max_value),
        }
    }

    fn get_offset(seed: i32, min_max_value: f32) -> Vector2 {
        let mut random = CommonRNG::from_seed(seed);

        Vector2 {
            x: random.f32(-min_max_value, min_max_value),
            y: random.f32(-min_max_value, min_max_value),
        }
    }
}
