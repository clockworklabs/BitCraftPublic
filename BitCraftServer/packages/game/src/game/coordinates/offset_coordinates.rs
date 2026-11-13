use super::hex_coordinates::HexCoordinates;

use std::convert::From;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct OffsetCoordinates {
    pub x: i32,
    pub z: i32,
    pub dimension: u32,
}

impl OffsetCoordinates {
    pub fn from_hashcode(hashcode: i64) -> Self {
        let x = (hashcode & 0xFFFF) as i32;
        let z = ((hashcode >> 16) & 0xFFFF) as i32;
        let dimension = ((hashcode >> 32) & 0xFFFF) as u32;
        Self { x, z, dimension }
    }

    pub fn hashcode(&self) -> i64 {
        ((self.x & 0xFFFF) | ((self.z & 0xFFFF) << 16)) as i64 | ((self.dimension as i64) << 32)
    }

    pub fn hashcode_long(&self) -> i128 {
        ((self.x as i128) & 0xFFFFFFFF) | (((self.z as i128) & 0xFFFFFFFF) << 32) | (((self.dimension as i128) & 0xFFFFFFFF) << 64)
    }

    pub fn scale(&self, s: f32) -> Self {
        Self::from(HexCoordinates::from(self).scale(s))
    }
}

impl From<HexCoordinates> for OffsetCoordinates {
    fn from(coordinates: HexCoordinates) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&HexCoordinates> for OffsetCoordinates {
    fn from(coordinates: &HexCoordinates) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}
