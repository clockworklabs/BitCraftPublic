use super::hex_coordinates::HexCoordinates;
use super::offset_coordinates::OffsetCoordinates;

use crate::game::coordinates::*;
use crate::game::entities::location::LocationState;
use std::convert::From;

impl From<HexCoordinates> for OffsetCoordinatesSmall {
    fn from(coordinates: HexCoordinates) -> Self {
        let oc: SmallHexTile = coordinates.into();
        let oc: OffsetCoordinatesSmall = oc.into();
        oc
    }
}

impl From<OffsetCoordinatesSmall> for HexCoordinates {
    fn from(coordinates: OffsetCoordinatesSmall) -> Self {
        let a: OffsetCoordinates = coordinates.into();
        a.into()
    }
}

impl From<OffsetCoordinatesSmall> for OffsetCoordinates {
    fn from(coordinates: OffsetCoordinatesSmall) -> Self {
        let hfc: SmallHexTile = coordinates.into();
        let hc: HexCoordinates = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinatesSmall> for OffsetCoordinates {
    fn from(coordinates: &OffsetCoordinatesSmall) -> Self {
        let hfc: SmallHexTile = coordinates.into();
        let hc: HexCoordinates = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinates> for OffsetCoordinatesSmall {
    fn from(coordinates: OffsetCoordinates) -> Self {
        let hfc: HexCoordinates = coordinates.into();
        let hc: SmallHexTile = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinates> for OffsetCoordinatesSmall {
    fn from(coordinates: &OffsetCoordinates) -> Self {
        let hfc: HexCoordinates = coordinates.into();
        let hc: SmallHexTile = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinatesSmall> for OffsetCoordinatesLarge {
    fn from(coordinates: OffsetCoordinatesSmall) -> Self {
        let hfc: SmallHexTile = coordinates.into();
        let hc: LargeHexTile = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinatesSmall> for OffsetCoordinatesLarge {
    fn from(coordinates: &OffsetCoordinatesSmall) -> Self {
        let hfc: SmallHexTile = coordinates.into();
        let hc: LargeHexTile = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinatesLarge> for OffsetCoordinatesSmall {
    fn from(coordinates: OffsetCoordinatesLarge) -> Self {
        let hfc: LargeHexTile = coordinates.into();
        let hc: SmallHexTile = hfc.into();
        hc.into()
    }
}

impl From<&OffsetCoordinatesLarge> for OffsetCoordinatesSmall {
    fn from(coordinates: &OffsetCoordinatesLarge) -> Self {
        let hfc: LargeHexTile = (*coordinates).into();
        let hc: SmallHexTile = hfc.into();
        hc.into()
    }
}

impl From<LocationState> for OffsetCoordinatesSmall {
    fn from(coordinates: LocationState) -> Self {
        Self {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&LocationState> for OffsetCoordinatesSmall {
    fn from(coordinates: &LocationState) -> Self {
        Self {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<OffsetCoordinatesSmall> for SmallHexTile {
    fn from(offset: OffsetCoordinatesSmall) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<&OffsetCoordinatesSmall> for SmallHexTile {
    fn from(offset: &OffsetCoordinatesSmall) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<SmallHexTile> for OffsetCoordinatesSmall {
    fn from(coordinates: SmallHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&SmallHexTile> for OffsetCoordinatesSmall {
    fn from(coordinates: &SmallHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl OffsetCoordinatesSmall {
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
