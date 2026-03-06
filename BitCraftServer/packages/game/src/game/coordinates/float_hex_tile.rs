use super::hex_coordinates::HexCoordinates;

use crate::game::coordinates::consts::*;
use crate::game::coordinates::*;
use crate::game::unity_helpers::vector2::Vector2;
use std::convert::From;
use std::fmt::Display;
use std::ops::Add;

impl From<FloatHexTile> for HexCoordinates {
    fn from(coordinates: FloatHexTile) -> Self {
        return Self::from_position(coordinates.to_world_position(), false, coordinates.dimension);
    }
}

impl From<&FloatHexTile> for HexCoordinates {
    fn from(coordinates: &FloatHexTile) -> Self {
        return Self::from_position(coordinates.to_world_position(), false, coordinates.dimension);
    }
}

impl From<HexCoordinates> for FloatHexTile {
    fn from(coordinates: HexCoordinates) -> Self {
        return Self::from_position(coordinates.to_center_position_xz(false), coordinates.dimension);
    }
}

impl From<&HexCoordinates> for FloatHexTile {
    fn from(coordinates: &HexCoordinates) -> Self {
        return Self::from_position(coordinates.to_center_position_xz(false), coordinates.dimension);
    }
}

impl From<FloatHexTile> for LargeHexTile {
    fn from(coordinates: FloatHexTile) -> Self {
        return LargeHexTile::from_position(coordinates.to_world_position(), coordinates.dimension);
    }
}

impl From<&FloatHexTile> for LargeHexTile {
    fn from(coordinates: &FloatHexTile) -> Self {
        return LargeHexTile::from_position(coordinates.to_world_position(), coordinates.dimension);
    }
}

impl From<FloatHexTile> for ChunkCoordinates {
    fn from(coordinates: FloatHexTile) -> Self {
        return LargeHexTile::from(coordinates).chunk_coordinates();
    }
}

impl From<&FloatHexTile> for ChunkCoordinates {
    fn from(coordinates: &FloatHexTile) -> Self {
        return LargeHexTile::from(coordinates).chunk_coordinates();
    }
}

impl Display for FloatHexTile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let oc = OffsetCoordinatesFloat::from(self);
        write!(
            f,
            "FloatHexTile ({}.{:.03}, {}.{:.03}, {})",
            oc.x / FLOAT_COORD_PRECISION_MUL,
            oc.x % FLOAT_COORD_PRECISION_MUL,
            oc.z / FLOAT_COORD_PRECISION_MUL,
            oc.z % FLOAT_COORD_PRECISION_MUL,
            oc.dimension
        )
    }
}

impl FloatHexTile {
    pub fn y(&self) -> i32 {
        -self.x - self.z
    }

    pub fn distance_to(&self, other: &FloatHexTile) -> f32 {
    // Positions in different dimensions should never interact.
    // Return UNREACHABLE_DISTANCE so any distance checks (distance < range) safely fail
    if self.dimension != other.dimension {
        return UNREACHABLE_DISTANCE_F32;
    }

    return ((((other.x - self.x).abs()
        + (other.y() - self.y()).abs()
        + (other.z - self.z).abs()) / 2) as f32)
        / FLOAT_COORD_PRECISION_MUL as f32;
    }

    pub fn to_world_position(&self) -> Vector2 {
        //Equivalent to FloatHexTile.ToCenterPositionVector2 on client

        let ix = (self.x as f32) / FLOAT_COORD_PRECISION_MUL as f32;
        let iz = (self.z as f32) / FLOAT_COORD_PRECISION_MUL as f32;

        let x = 2.0 * ix * INNER_RADIUS + iz * INNER_RADIUS;
        let z = 1.5 * iz * OUTER_RADIUS;

        return Vector2 { x, y: z };
    }

    pub fn from_position(world_position: Vector2, dimension: u32) -> FloatHexTile {
        let mut x = world_position.x / (INNER_RADIUS * 2.0);
        let mut y = -x;
        let offset = world_position.y / (OUTER_RADIUS * 3.0);
        x -= offset;
        y -= offset;
        let mut z = -x - y;
        x += 0.5 / FLOAT_COORD_PRECISION_MUL as f32;
        z += 0.5 / FLOAT_COORD_PRECISION_MUL as f32;

        let oc = FloatHexTile {
            x: (x * FLOAT_COORD_PRECISION_MUL as f32) as i32,
            z: (z * FLOAT_COORD_PRECISION_MUL as f32) as i32,
            dimension,
        };
        return oc;
    }

    pub fn parent_large_tile(&self) -> LargeHexTile {
        return LargeHexTile::from(self);
    }

    pub fn parent_small_tile(&self) -> SmallHexTile {
        return SmallHexTile::from(self);
    }

    pub fn parent_small_and_large_tile(&self) -> (SmallHexTile, LargeHexTile) {
        let d = self.dimension;
        let p = self.to_world_position();
        return (SmallHexTile::from_position(p.clone(), d), LargeHexTile::from_position(p, d));
    }

    pub fn chunk_coordinates(&self) -> ChunkCoordinates {
        return ChunkCoordinates::from(self);
    }

    pub fn lerp(a: &FloatHexTile, b: &FloatHexTile, t: f32) -> FloatHexTile {
        let av = a.to_world_position();
        let bv = b.to_world_position();
        let lv = Vector2::lerp(&av, &bv, t);
        return Self::from_position(lv, a.dimension);
    }
}


impl Add for FloatHexTile {
    type Output = FloatHexTile;

    fn add(self, rhs: Self) -> Self::Output {
        FloatHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}