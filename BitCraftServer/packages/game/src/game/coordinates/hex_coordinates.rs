use super::hex_direction::HexDirection;
use super::offset_coordinates::OffsetCoordinates;

use crate::game::coordinates::consts::*;
use crate::game::unity_helpers::vector2::Vector2;
use std::convert::From;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Sub};

#[derive(Default, Clone, PartialEq)]
pub struct HexCoordinates {
    pub x: i32,
    pub z: i32,
    pub dimension: u32,
}

impl Copy for HexCoordinates {}

impl Sub for HexCoordinates {
    type Output = HexCoordinates;

    fn sub(self, rhs: Self) -> Self::Output {
        HexCoordinates {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Sub for &HexCoordinates {
    type Output = HexCoordinates;

    fn sub(self, rhs: Self) -> Self::Output {
        HexCoordinates {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for &HexCoordinates {
    type Output = HexCoordinates;

    fn add(self, rhs: Self) -> Self::Output {
        HexCoordinates {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for HexCoordinates {
    type Output = HexCoordinates;

    fn add(self, rhs: Self) -> Self::Output {
        HexCoordinates {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<HexCoordinates> for &HexCoordinates {
    type Output = HexCoordinates;

    fn add(self, rhs: HexCoordinates) -> Self::Output {
        HexCoordinates {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<&Self> for HexCoordinates {
    type Output = HexCoordinates;

    fn add(self, rhs: &Self) -> Self::Output {
        HexCoordinates {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl HexCoordinates {
    pub fn zero() -> Self {
        Self { x: 0, z: 0, dimension: 0 }
    }

    pub fn y(&self) -> i32 {
        -self.x - self.z
    }

    pub fn from_hashcode(hashcode: i64) -> Self {
        let offset = OffsetCoordinates::from_hashcode(hashcode);
        HexCoordinates::from(&offset)
    }

    pub fn hashcode(&self) -> i64 {
        return OffsetCoordinates::from(self).hashcode();
    }

    pub fn hashcode_long(&self) -> i128 {
        return OffsetCoordinates::from(self).hashcode_long();
    }

    pub fn scale(&self, s: f32) -> Self {
        Self {
            x: (self.x as f32 * s).round() as i32,
            z: (self.z as f32 * s).round() as i32,
            dimension: self.dimension,
        }
    }

    pub fn neighbor_n(&self, direction: HexDirection, n: i32) -> HexCoordinates {
        let (xoffset, zoffset) = match direction {
            HexDirection::NE => (0, 1),
            HexDirection::ENE => (1, 1),
            HexDirection::E => (1, 0),
            HexDirection::ESE => (2, -1),
            HexDirection::SE => (1, -1),
            HexDirection::S => (1, -2),
            HexDirection::SW => (0, -1),
            HexDirection::WSW => (-1, -1),
            HexDirection::W => (-1, 0),
            HexDirection::WNW => (-2, 1),
            HexDirection::NW => (-1, 1),
            HexDirection::N => (-1, 2),
        };
        HexCoordinates {
            x: self.x + n * xoffset,
            z: self.z + n * zoffset,
            dimension: self.dimension,
        }
    }

    pub fn neighbor(&self, direction: HexDirection) -> HexCoordinates {
        self.neighbor_n(direction, 1)
    }

    pub fn direction(&self, neighbor: HexCoordinates) -> Option<HexDirection> {
        let xoffset = neighbor.x - self.x;
        let zoffset = neighbor.z - self.z;
        match (xoffset, zoffset) {
            (0, 1) => Some(HexDirection::NE),
            (1, 1) => Some(HexDirection::ENE),
            (1, 0) => Some(HexDirection::E),
            (2, -1) => Some(HexDirection::ESE),
            (1, -1) => Some(HexDirection::SE),
            (1, -2) => Some(HexDirection::S),
            (0, -1) => Some(HexDirection::SW),
            (-1, -1) => Some(HexDirection::WSW),
            (-1, 0) => Some(HexDirection::W),
            (-2, 1) => Some(HexDirection::WNW),
            (-1, 1) => Some(HexDirection::NW),
            (-1, 2) => Some(HexDirection::N),
            _ => None,
        }
    }

    pub fn rotate_around(&self, center: &HexCoordinates, steps: i32) -> HexCoordinates {
        let mut p_from_c = self - center;
        for _ in 0..(steps % 6) {
            p_from_c = HexCoordinates {
                x: -p_from_c.y(),
                z: -p_from_c.x,
                dimension: self.dimension,
            };
        }
        for _ in 0..-(steps % 6) {
            p_from_c = HexCoordinates {
                x: -p_from_c.z,
                z: -p_from_c.y(),
                dimension: self.dimension,
            };
        }
        p_from_c + center
    }

    pub fn distance_to(&self, other: HexCoordinates) -> i32 {
    // Positions in different dimensions should never interact.
    // Return UNREACHABLE_DISTANCE so any distance checks (distance < range) safely fail
    if self.dimension != other.dimension {
        return UNREACHABLE_DISTANCE_I32;
    }

    return ((other.x - self.x).abs()
        + (other.y() - self.y()).abs()
        + (other.z - self.z).abs()) / 2;
}

    pub fn get_terrain_coordinates(&self) -> [HexCoordinates; 3] {
        // There technically are two types of corner cells based
        // on whether the terrain cell borders intersect them one way or the other.
        let center = self.scale(1. / 3.).scale(3.0);
        let xdiff = self.x - center.x;
        let zdiff = self.z - center.z;

        let is_up_corner = (xdiff == -1 && (zdiff == -1 || zdiff == 2)) || (xdiff == 2 && zdiff == -1);
        let is_down_corner = (xdiff == 1 && (zdiff == -2 || zdiff == 1)) || (xdiff == -2 && zdiff == 1);
        if is_up_corner {
            let xcoords = self
                + HexCoordinates {
                    x: -2,
                    z: 1,
                    dimension: self.dimension,
                };
            let ycoords = self
                + HexCoordinates {
                    x: 1,
                    z: -2,
                    dimension: self.dimension,
                };
            let zcoords = self
                + HexCoordinates {
                    x: 1,
                    z: 1,
                    dimension: self.dimension,
                };
            let xtcoords = xcoords.scale(1. / 3.);
            let ytcoords = ycoords.scale(1. / 3.);
            let ztcoords = zcoords.scale(1. / 3.);
            return [xtcoords, ytcoords, ztcoords];
        }

        if is_down_corner {
            let xcoords = self
                + HexCoordinates {
                    x: 2,
                    z: -1,
                    dimension: self.dimension,
                };
            let ycoords = self
                + HexCoordinates {
                    x: -1,
                    z: 2,
                    dimension: self.dimension,
                };
            let zcoords = self
                + HexCoordinates {
                    x: -1,
                    z: -1,
                    dimension: self.dimension,
                };
            let xtcoords = xcoords.scale(1. / 3.);
            let ytcoords = ycoords.scale(1. / 3.);
            let ztcoords = zcoords.scale(1. / 3.);
            return [xtcoords, ytcoords, ztcoords];
        }

        let c = self.scale(1. / 3.);
        return [c, c, c];
    }

    pub fn is_corner(&self) -> bool {
        let mut x = self.x % 3;
        x = (x + 3) % 3; //This handles negative numbers without branching
        let mut z = self.z % 3;
        z = (z + 3) % 3; //This handles negative numbers without branching
        return (x == z) & (x != 0);
    }

    pub fn approximate_direction(&self, coordinates: HexCoordinates) -> HexDirection {
        let outer_radius = 1.0;
        let inner_radius = 0.866025404;

        let position = (
            2.0 * self.x as f64 * inner_radius + self.z as f64 * inner_radius,
            1.5 * self.z as f64 * outer_radius,
        );
        let other = (
            2.0 * coordinates.x as f64 * inner_radius + coordinates.z as f64 * inner_radius,
            1.5 * coordinates.z as f64 * outer_radius,
        );

        let vector = (other.0 - position.0, other.1 - position.1);
        let angle = f64::atan2(vector.0, vector.1);
        HexDirection::radians_to_direction(angle)
    }

    pub fn angle(&self, coordinates: HexCoordinates) -> f64 {
        let outer_radius = 1.0;
        let inner_radius = 0.866025404;

        let position = (
            2.0 * self.x as f64 * inner_radius + self.z as f64 * inner_radius,
            1.5 * self.z as f64 * outer_radius,
        );
        let other = (
            2.0 * coordinates.x as f64 * inner_radius + coordinates.z as f64 * inner_radius,
            1.5 * coordinates.z as f64 * outer_radius,
        );

        let vector = (other.0 - position.0, other.1 - position.1);
        f64::atan2(vector.0, vector.1)
    }

    pub fn coordinates_in_radius(center: HexCoordinates, radius: i32) -> Vec<HexCoordinates> {
        let mut list = Vec::new();
        for i in 1..=radius {
            list.extend_from_slice(&HexCoordinates::ring(center, i));
        }
        list //Doesn't include center
    }

    pub fn coordinates_in_radius_with_center_iter(center: HexCoordinates, radius: i32) -> impl Iterator<Item = HexCoordinates> {
        //DOES include center
        (1..=radius)
            .into_iter()
            .map(move |i| HexCoordinates::ring_iter(center, i))
            .flatten()
            .chain(std::iter::once(center))
    }

    pub fn ring(center: HexCoordinates, radius: i32) -> Vec<HexCoordinates> {
        if radius <= 0 {
            return vec![center];
        }

        let mut list = Vec::new();
        let mut direction = HexDirection::NE;
        let mut coordinates = center.neighbor_n(HexDirection::next_flat(HexDirection::next_flat(direction)), radius);
        for _j in 0..6 {
            for _k in 0..radius {
                coordinates = coordinates.neighbor(direction);
                list.push(coordinates);
            }
            direction = HexDirection::previous_flat(direction);
        }
        list
    }

    pub fn ring_iter(center: HexCoordinates, radius: i32) -> impl Iterator<Item = HexCoordinates> {
        return HexCoordinatesRingIter::new(center, radius);
    }

    pub fn closest(&self, locations: &Vec<HexCoordinates>) -> Option<HexCoordinates> {
        let mut shortest_distance = i32::MAX;
        let mut closest = None;
        for coordinates in locations {
            let distance = self.distance_to(*coordinates);
            if distance < shortest_distance {
                shortest_distance = distance;
                closest = Some(*coordinates);
            }
        }
        return closest;
    }

    pub fn is_center(&self) -> bool {
        return self.x % 3 == 0 && self.z % 3 == 0;
    }

    pub fn to_offset_coordinates(&self) -> OffsetCoordinates {
        self.into()
    }

    pub fn from_offset_coordinates(x: i32, z: i32, dimension: u32) -> HexCoordinates {
        return HexCoordinates {
            x: x - z / 2,
            z: z,
            dimension,
        };
    }

    pub fn to_center_position_xz(&self, terrain: bool) -> Vector2 {
        let i_x = self.x as f32;
        let i_z = self.z as f32;

        let inner_radius = if terrain { TERRAIN_INNER_RADIUS } else { INNER_RADIUS };
        let outer_radius = if terrain { TERRAIN_OUTER_RADIUS } else { OUTER_RADIUS };

        let x = 2.0f32 * i_x * inner_radius + i_z * inner_radius;
        let z = 1.5f32 * i_z * outer_radius;
        return Vector2 { x, y: z };
    }

    pub fn from_position(position: Vector2, terrain: bool, dimension: u32) -> HexCoordinates {
        //Equivalent to HexCoordinates.FromPosition on client

        let inner_radius = if terrain { TERRAIN_INNER_RADIUS } else { INNER_RADIUS };
        let outer_radius = if terrain { TERRAIN_OUTER_RADIUS } else { OUTER_RADIUS };

        let mut x = position.x / (inner_radius * 2.0);
        let mut y = -x;
        let offset = position.y / (outer_radius * 3.0);
        x -= offset;
        y -= offset;
        let mut ix = x.round() as i32;
        let iy = y.round() as i32;
        let mut iz = (-x - y).round() as i32;

        if ix + iy + iz != 0 {
            let dx = (x - ix as f32).abs();
            let dy = (y - iy as f32).abs();
            let dz = (-x - y - iz as f32).abs();

            if dx > dy && dx > dz {
                ix = -iy - iz;
            } else if dz > dy {
                iz = -ix - iy;
            }
        }

        return HexCoordinates { x: ix, z: iz, dimension };
    }
}

impl From<OffsetCoordinates> for HexCoordinates {
    fn from(offset: OffsetCoordinates) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl From<&OffsetCoordinates> for HexCoordinates {
    fn from(offset: &OffsetCoordinates) -> Self {
        Self {
            x: offset.x - offset.z / 2,
            z: offset.z,
            dimension: offset.dimension,
        }
    }
}

impl Hash for HexCoordinates {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(self.hashcode());
    }
}

impl Display for HexCoordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let oc = OffsetCoordinates::from(self);
        write!(f, "HexCoordinates ({}, {}, {})", oc.x, oc.z, oc.dimension)
    }
}

struct HexCoordinatesRingIter {
    pub direction: HexDirection,
    pub coordinate: HexCoordinates,
    pub radius: i32,
    pub j: i32,
    pub k: i32,
}

impl HexCoordinatesRingIter {
    pub fn new(center: HexCoordinates, radius: i32) -> Self {
        if radius <= 0 {
            return HexCoordinatesRingIter {
                direction: HexDirection::NE,
                coordinate: center,
                radius,
                j: 0,
                k: 0,
            };
        }

        HexCoordinatesRingIter {
            direction: HexDirection::NE,
            coordinate: center.neighbor_n(HexDirection::next_flat(HexDirection::next_flat(HexDirection::NE)), radius),
            radius,
            j: 0,
            k: -1,
        }
    }
}

impl Iterator for HexCoordinatesRingIter {
    type Item = HexCoordinates;

    fn next(&mut self) -> Option<Self::Item> {
        if self.radius <= 0 && self.k == 0 {
            self.j = 10; //This will force iter to terminate on next call
            return Some(self.coordinate); //Return center coord
        }

        self.k += 1;
        if self.k >= self.radius {
            self.j += 1;
            if self.j >= 6 {
                return None;
            }
            self.k = 0;
            self.direction = HexDirection::previous_flat(self.direction);
        }

        self.coordinate = self.coordinate.neighbor(self.direction);
        return Some(self.coordinate);
    }
}
