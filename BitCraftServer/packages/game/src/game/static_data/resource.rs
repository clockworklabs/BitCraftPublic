use crate::game::coordinates::*;
use crate::messages::static_data::*;

impl ResourceDesc {
    pub fn get_footprint(&self, coordinates: &SmallHexTile, direction: i32) -> Vec<(SmallHexTile, FootprintType)> {
        self.footprint
            .iter()
            .map(|delta| {
                (
                    SmallHexTile {
                        x: coordinates.x + delta.x,
                        z: coordinates.z + delta.z,
                        dimension: coordinates.dimension,
                    }
                    .rotate_around(coordinates, direction / 2),
                    delta.footprint_type,
                )
            })
            .collect()
    }

    pub fn get_footprint_radius(&self) -> i32 {
        if !self.footprint.is_empty() {
            let mut max_hex_offset = 1;
            for footprint_delta in &self.footprint {
                if footprint_delta.footprint_type != FootprintType::Perimeter {
                    if footprint_delta.x.abs() > max_hex_offset {
                        max_hex_offset = footprint_delta.x.abs();
                    }
                    if footprint_delta.z.abs() > max_hex_offset {
                        max_hex_offset = footprint_delta.z.abs();
                    }
                }
                //log::debug!("Footprint delta {} {}", footprint_delta.x, footprint_delta.z);
            }
            return max_hex_offset;
        }

        1
    }
}
