use super::hex_coordinates::HexCoordinates;
use super::offset_coordinates::OffsetCoordinates;

use crate::game::coordinates::*;
use crate::messages::components::MobileEntityState;
use std::convert::From;

impl From<(i32, i32, u32)> for OffsetCoordinatesFloat {
    fn from((x, z, dimension): (i32, i32, u32)) -> Self {
        Self { x, z, dimension }
    }
}

impl From<HexCoordinates> for OffsetCoordinatesFloat {
    fn from(coordinates: HexCoordinates) -> Self {
        let oc: FloatHexTile = coordinates.into();
        let oc: OffsetCoordinatesFloat = oc.into();
        oc
    }
}

impl From<OffsetCoordinatesFloat> for OffsetCoordinates {
    fn from(coordinates: OffsetCoordinatesFloat) -> Self {
        let hfc: FloatHexTile = coordinates.into();
        let hc: HexCoordinates = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinates> for OffsetCoordinatesFloat {
    fn from(coordinates: OffsetCoordinates) -> Self {
        let hfc: HexCoordinates = coordinates.into();
        let hc: FloatHexTile = hfc.into();
        hc.into()
    }
}

impl From<OffsetCoordinatesFloat> for HexCoordinates {
    fn from(coordinates: OffsetCoordinatesFloat) -> Self {
        let a: OffsetCoordinates = coordinates.into();
        a.into()
    }
}

impl From<MobileEntityState> for OffsetCoordinatesFloat {
    fn from(coordinates: MobileEntityState) -> Self {
        Self {
            x: coordinates.location_x,
            z: coordinates.location_z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&MobileEntityState> for OffsetCoordinatesFloat {
    fn from(coordinates: &MobileEntityState) -> Self {
        Self {
            x: coordinates.location_x,
            z: coordinates.location_z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<OffsetCoordinatesFloat> for FloatHexTile {
    fn from(offset: OffsetCoordinatesFloat) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<&OffsetCoordinatesFloat> for FloatHexTile {
    fn from(offset: &OffsetCoordinatesFloat) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<FloatHexTile> for OffsetCoordinatesFloat {
    fn from(coordinates: FloatHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<&FloatHexTile> for OffsetCoordinatesFloat {
    fn from(coordinates: &FloatHexTile) -> Self {
        Self {
            x: coordinates.x + coordinates.z / 2,
            z: coordinates.z,
            dimension: coordinates.dimension,
        }
    }
}

impl From<OffsetCoordinatesFloat> for OffsetCoordinatesSmall {
    fn from(coordinates: OffsetCoordinatesFloat) -> Self {
        OffsetCoordinatesSmall::from(SmallHexTile::from(FloatHexTile::from(coordinates)))
    }
}

impl From<&OffsetCoordinatesFloat> for OffsetCoordinatesSmall {
    fn from(coordinates: &OffsetCoordinatesFloat) -> Self {
        OffsetCoordinatesSmall::from(SmallHexTile::from(FloatHexTile::from(*coordinates)))
    }
}

impl From<OffsetCoordinatesSmall> for OffsetCoordinatesFloat {
    fn from(coordinates: OffsetCoordinatesSmall) -> Self {
        OffsetCoordinatesFloat::from(FloatHexTile::from(SmallHexTile::from(coordinates)))
    }
}

impl From<&OffsetCoordinatesSmall> for OffsetCoordinatesFloat {
    fn from(coordinates: &OffsetCoordinatesSmall) -> Self {
        OffsetCoordinatesFloat::from(FloatHexTile::from(SmallHexTile::from(*coordinates)))
    }
}

impl From<SmallHexTile> for OffsetCoordinatesFloat {
    fn from(coordinates: SmallHexTile) -> Self {
        OffsetCoordinatesFloat::from(FloatHexTile::from(coordinates))
    }
}

impl From<&SmallHexTile> for OffsetCoordinatesFloat {
    fn from(coordinates: &SmallHexTile) -> Self {
        OffsetCoordinatesFloat::from(FloatHexTile::from(*coordinates))
    }
}
