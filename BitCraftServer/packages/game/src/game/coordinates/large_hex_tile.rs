use super::hex_coordinates::HexCoordinates;

use crate::game::coordinates::*;
use crate::game::unity_helpers::vector2::Vector2;
use std::convert::From;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Sub};

impl From<LargeHexTile> for HexCoordinates {
    fn from(coordinates: LargeHexTile) -> Self {
        HexCoordinates {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&LargeHexTile> for HexCoordinates {
    fn from(coordinates: &LargeHexTile) -> Self {
        HexCoordinates {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<HexCoordinates> for LargeHexTile {
    fn from(coordinates: HexCoordinates) -> Self {
        LargeHexTile {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&HexCoordinates> for LargeHexTile {
    fn from(coordinates: &HexCoordinates) -> Self {
        LargeHexTile {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<LargeHexTile> for ChunkCoordinates {
    fn from(coordinates: LargeHexTile) -> Self {
        ChunkCoordinates::from(OffsetCoordinatesLarge::from(coordinates))
    }
}

impl From<&LargeHexTile> for ChunkCoordinates {
    fn from(coordinates: &LargeHexTile) -> Self {
        ChunkCoordinates::from(OffsetCoordinatesLarge::from(coordinates))
    }
}

impl Sub for LargeHexTile {
    type Output = LargeHexTile;

    fn sub(self, rhs: Self) -> Self::Output {
        LargeHexTile {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Sub for &LargeHexTile {
    type Output = LargeHexTile;

    fn sub(self, rhs: Self) -> Self::Output {
        LargeHexTile {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for &LargeHexTile {
    type Output = LargeHexTile;

    fn add(self, rhs: Self) -> Self::Output {
        LargeHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for LargeHexTile {
    type Output = LargeHexTile;

    fn add(self, rhs: Self) -> Self::Output {
        LargeHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<LargeHexTile> for &LargeHexTile {
    type Output = LargeHexTile;

    fn add(self, rhs: LargeHexTile) -> Self::Output {
        LargeHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<&Self> for LargeHexTile {
    type Output = LargeHexTile;

    fn add(self, rhs: &Self) -> Self::Output {
        LargeHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Hash for LargeHexTile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(self.hashcode());
    }
}

impl Display for LargeHexTile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let oc = OffsetCoordinatesLarge::from(self);
        write!(f, "LargeHexTile ({}, {}, {})", oc.x, oc.z, oc.dimension)
    }
}

impl LargeHexTile {
    pub fn center_small_tile(&self) -> SmallHexTile {
        SmallHexTile::from(self)
    }

    pub fn chunk_coordinates(&self) -> ChunkCoordinates {
        ChunkCoordinates::from(self)
    }

    pub fn to_offset_coordinates(&self) -> OffsetCoordinatesLarge {
        (*self).into()
    }

    pub fn from_hashcode(hashcode: i64) -> Self {
        let offset = OffsetCoordinatesLarge::from_hashcode(hashcode);
        LargeHexTile::from(&offset)
    }

    pub fn hashcode(&self) -> i64 {
        OffsetCoordinatesLarge::from(self).hashcode()
    }

    pub fn hashcode_long(&self) -> i128 {
        OffsetCoordinatesLarge::from(self).hashcode_long()
    }

    pub fn neighbor_n(&self, direction: HexDirection, n: i32) -> LargeHexTile {
        HexCoordinates::from(self).neighbor_n(direction, n).into()
    }

    pub fn neighbor(&self, direction: HexDirection) -> LargeHexTile {
        HexCoordinates::from(self).neighbor(direction).into()
    }

    pub fn direction(&self, neighbor: &LargeHexTile) -> Option<HexDirection> {
        HexCoordinates::from(self).direction(neighbor.into())
    }

    pub fn rotate_around(&self, center: &LargeHexTile, steps: i32) -> LargeHexTile {
        HexCoordinates::from(self)
            .rotate_around(&HexCoordinates::from(center), steps)
            .into()
    }

    pub fn distance_to(&self, other: LargeHexTile) -> i32 {
        if other.dimension != self.dimension {
            return i32::MAX;
        }
        HexCoordinates::from(self).distance_to(other.into())
    }

    pub fn is_corner(&self) -> bool {
        HexCoordinates::from(self).is_corner()
    }

    pub fn approximate_direction(&self, coordinates: LargeHexTile) -> HexDirection {
        HexCoordinates::from(self).approximate_direction(coordinates.into())
    }

    pub fn angle(&self, coordinates: LargeHexTile) -> f64 {
        HexCoordinates::from(self).angle(coordinates.into())
    }

    pub fn coordinates_in_radius(center: LargeHexTile, radius: i32) -> Vec<LargeHexTile> {
        //Doesn't include center
        HexCoordinates::coordinates_in_radius(center.into(), radius)
            .iter()
            .map(LargeHexTile::from)
            .collect()
    }

    pub fn ring(center: LargeHexTile, radius: i32) -> Vec<LargeHexTile> {
        HexCoordinates::ring(center.into(), radius)
            .iter()
            .map(LargeHexTile::from)
            .collect()
    }

    pub fn closest(&self, locations: &[LargeHexTile]) -> Option<LargeHexTile> {
        let locations = locations.iter().map(HexCoordinates::from).collect();
        {
            let a = HexCoordinates::from(self).closest(&locations)?;
            Some(LargeHexTile::from(a))
        }
    }

    pub fn is_center(&self) -> bool {
        HexCoordinates::from(self).is_center()
    }

    pub fn to_center_position_xz(&self) -> Vector2 {
        HexCoordinates::from(self).to_center_position_xz(true)
    }

    pub fn from_position(position: Vector2, dimension: u32) -> LargeHexTile {
        LargeHexTile::from(HexCoordinates::from_position(position, true, dimension))
    }
}
