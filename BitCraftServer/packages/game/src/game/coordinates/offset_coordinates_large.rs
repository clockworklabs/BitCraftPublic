use super::hex_coordinates::HexCoordinates;
use super::offset_coordinates::OffsetCoordinates;

use crate::game::coordinates::*;
use std::convert::From;

impl From<HexCoordinates> for OffsetCoordinatesLarge {
    fn from(coordinates: HexCoordinates) -> Self {
        let oc: LargeHexTile = coordinates.into();
        let oc: OffsetCoordinatesLarge = oc.into();
        oc
    }
}

impl From<OffsetCoordinatesLarge> for HexCoordinates {
    fn from(coordinates: OffsetCoordinatesLarge) -> Self {
        let a: OffsetCoordinates = coordinates.into();
        a.into()
    }
}

impl From<OffsetCoordinatesLarge> for OffsetCoordinates {
    fn from(coordinates: OffsetCoordinatesLarge) -> Self {
        let hfc: LargeHexTile = coordinates.into();
        let hc: HexCoordinates = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinatesLarge> for OffsetCoordinates {
    fn from(coordinates: &OffsetCoordinatesLarge) -> Self {
        let hfc: LargeHexTile = coordinates.into();
        let hc: HexCoordinates = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinates> for OffsetCoordinatesLarge {
    fn from(coordinates: OffsetCoordinates) -> Self {
        let hfc: HexCoordinates = coordinates.into();
        let hc: LargeHexTile = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinates> for OffsetCoordinatesLarge {
    fn from(coordinates: &OffsetCoordinates) -> Self {
        let hfc: HexCoordinates = coordinates.into();
        let hc: LargeHexTile = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinatesLarge> for LargeHexTile {
    fn from(offset: OffsetCoordinatesLarge) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<&OffsetCoordinatesLarge> for LargeHexTile {
    fn from(offset: &OffsetCoordinatesLarge) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<LargeHexTile> for OffsetCoordinatesLarge {
    fn from(coordinates: LargeHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&LargeHexTile> for OffsetCoordinatesLarge {
    fn from(coordinates: &LargeHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl OffsetCoordinatesLarge {
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
}
