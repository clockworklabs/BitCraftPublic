use super::super::unity_helpers::vector2::Vector2;

use super::land_shape_definition::LandShapeDefinition;

#[derive(Debug)]
pub struct WorldMapDefinition {
    pub debug_step: i32,
    pub shapes: Vec<LandShapeDefinition>,
}

impl WorldMapDefinition {
    pub fn is_water(&self, position: Vector2) -> bool {
        self.is_water_internal(position)
    }

    pub fn get_noise(&self, position: Vector2) -> f32 {
        self.get_noise_internal(position)
    }

    fn is_water_internal(&self, position: Vector2) -> bool {
        if self.shapes.is_empty() {
            return true;
        }

        for shape in &self.shapes {
            if !shape.is_water_vec2(position) {
                return false;
            }
        }

        true
    }

    fn get_noise_internal(&self, position: Vector2) -> f32 {
        if self.shapes.is_empty() {
            return 0f32;
        }

        let water = self.is_water_internal(position);

        let mut noise = 0f32;
        for shape in &self.shapes {
            if water != shape.is_water_vec2(position) {
                continue;
            }
            noise = noise.max(shape.get_vec2(position));
        }

        noise
    }
}
