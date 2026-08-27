use hex_direction::HexDirection;
use spacetimedb::ReducerContext;

use super::hex_coordinates::HexCoordinates;

use crate::game::coordinates::*;
use crate::game::unity_helpers::vector2::Vector2;
use crate::FootprintType;
use spacetimedb::rand::seq::SliceRandom;
use spacetimedb::rand::Rng;
use std::collections::HashSet;
use std::convert::From;
use std::fmt::Display;
use std::hash::Hash;
use std::i32;
use std::ops::{Add, Sub};

impl From<SmallHexTile> for HexCoordinates {
    fn from(coordinates: SmallHexTile) -> Self {
        return HexCoordinates {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        };
    }
}

impl From<&SmallHexTile> for HexCoordinates {
    fn from(coordinates: &SmallHexTile) -> Self {
        return HexCoordinates {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        };
    }
}

impl From<HexCoordinates> for SmallHexTile {
    fn from(coordinates: HexCoordinates) -> Self {
        return SmallHexTile {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        };
    }
}

impl From<&HexCoordinates> for SmallHexTile {
    fn from(coordinates: &HexCoordinates) -> Self {
        return SmallHexTile {
            x: coordinates.x,
            z: coordinates.z,
            dimension: coordinates.dimension,
        };
    }
}

impl From<SmallHexTile> for LargeHexTile {
    fn from(coordinates: SmallHexTile) -> Self {
        let mut hc = HexCoordinates::from(coordinates);
        hc = hc.scale(1. / 3.);
        return LargeHexTile::from(hc);
    }
}

impl From<&SmallHexTile> for LargeHexTile {
    fn from(coordinates: &SmallHexTile) -> Self {
        let mut hc = HexCoordinates::from(coordinates);
        hc = hc.scale(1. / 3.);
        return LargeHexTile::from(hc);
    }
}

impl From<LargeHexTile> for SmallHexTile {
    fn from(coordinates: LargeHexTile) -> Self {
        let mut hc = HexCoordinates::from(coordinates);
        hc = hc.scale(3.);
        return SmallHexTile::from(hc);
    }
}

impl From<&LargeHexTile> for SmallHexTile {
    fn from(coordinates: &LargeHexTile) -> Self {
        let mut hc = HexCoordinates::from(coordinates);
        hc = hc.scale(3.);
        return SmallHexTile::from(hc);
    }
}

impl From<SmallHexTile> for FloatHexTile {
    fn from(coordinates: SmallHexTile) -> Self {
        return FloatHexTile::from(HexCoordinates::from(coordinates));
    }
}

impl From<&SmallHexTile> for FloatHexTile {
    fn from(coordinates: &SmallHexTile) -> Self {
        return FloatHexTile::from(HexCoordinates::from(coordinates));
    }
}

impl From<FloatHexTile> for SmallHexTile {
    fn from(coordinates: FloatHexTile) -> Self {
        return SmallHexTile::from(HexCoordinates::from(coordinates));
    }
}

impl From<&FloatHexTile> for SmallHexTile {
    fn from(coordinates: &FloatHexTile) -> Self {
        return SmallHexTile::from(HexCoordinates::from(coordinates));
    }
}

impl From<SmallHexTile> for ChunkCoordinates {
    fn from(coordinates: SmallHexTile) -> Self {
        return ChunkCoordinates::from(HexCoordinates::from(coordinates));
    }
}

impl From<&SmallHexTile> for ChunkCoordinates {
    fn from(coordinates: &SmallHexTile) -> Self {
        return ChunkCoordinates::from(HexCoordinates::from(coordinates));
    }
}

impl Sub for SmallHexTile {
    type Output = SmallHexTile;

    fn sub(self, rhs: Self) -> Self::Output {
        SmallHexTile {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Sub for &SmallHexTile {
    type Output = SmallHexTile;

    fn sub(self, rhs: Self) -> Self::Output {
        SmallHexTile {
            x: self.x - rhs.x,
            z: self.z - rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for &SmallHexTile {
    type Output = SmallHexTile;

    fn add(self, rhs: Self) -> Self::Output {
        SmallHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add for SmallHexTile {
    type Output = SmallHexTile;

    fn add(self, rhs: Self) -> Self::Output {
        SmallHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<SmallHexTile> for &SmallHexTile {
    type Output = SmallHexTile;

    fn add(self, rhs: SmallHexTile) -> Self::Output {
        SmallHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Add<&Self> for SmallHexTile {
    type Output = SmallHexTile;

    fn add(self, rhs: &Self) -> Self::Output {
        SmallHexTile {
            x: self.x + rhs.x,
            z: self.z + rhs.z,
            dimension: self.dimension,
        }
    }
}

impl Hash for SmallHexTile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(self.hashcode());
    }
}

impl Display for SmallHexTile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let oc = OffsetCoordinatesSmall::from(self);
        write!(f, "SmallHexTile ({}, {}, {})", oc.x, oc.z, oc.dimension)
    }
}

impl SmallHexTile {
    pub fn parent_large_tile(&self) -> LargeHexTile {
        return LargeHexTile::from(self);
    }

    pub fn chunk_coordinates(&self) -> ChunkCoordinates {
        return ChunkCoordinates::from(self);
    }

    pub fn to_offset_coordinates(&self) -> OffsetCoordinatesSmall {
        return self.into();
    }

    pub fn from_hashcode(hashcode: i64) -> Self {
        let offset = OffsetCoordinatesSmall::from_hashcode(hashcode);
        SmallHexTile::from(&offset)
    }

    pub fn hashcode(&self) -> i64 {
        return OffsetCoordinatesSmall::from(self).hashcode();
    }

    pub fn hashcode_long(&self) -> i128 {
        return OffsetCoordinatesSmall::from(self).hashcode_long();
    }

    pub fn neighbor_n(&self, direction: HexDirection, n: i32) -> SmallHexTile {
        return HexCoordinates::from(self).neighbor_n(direction, n).into();
    }

    pub fn neighbor(&self, direction: HexDirection) -> SmallHexTile {
        return HexCoordinates::from(self).neighbor(direction).into();
    }

    pub fn neighbors_direct(&self) -> impl Iterator<Item = SmallHexTile> + '_ {
        return HexDirection::FLAT.iter().map(|d| self.neighbor(*d));
    }

    pub fn direction(&self, neighbor: SmallHexTile) -> Option<HexDirection> {
        return HexCoordinates::from(self).direction(neighbor.into());
    }

    pub fn rotate_around(&self, center: &SmallHexTile, steps: i32) -> SmallHexTile {
        return HexCoordinates::from(self)
            .rotate_around(&HexCoordinates::from(center), steps)
            .into();
    }

    pub fn distance_to(&self, other: SmallHexTile) -> i32 {
        if other.dimension != self.dimension {
            return i32::MAX;
        }
        return HexCoordinates::from(self).distance_to(other.into());
    }

    pub fn distance_to_footprint(&self, footprint: Vec<(SmallHexTile, FootprintType)>) -> i32 {
        let mut distance = i32::MAX;

        for (coords, _) in footprint {
            distance = distance.min(self.distance_to(coords));
        }

        distance
    }

    pub fn is_corner(&self) -> bool {
        return HexCoordinates::from(self).is_corner();
    }

    pub fn approximate_direction(&self, coordinates: SmallHexTile) -> HexDirection {
        return HexCoordinates::from(self).approximate_direction(coordinates.into());
    }

    pub fn angle(&self, coordinates: SmallHexTile) -> f64 {
        return HexCoordinates::from(self).angle(coordinates.into());
    }

    pub fn coordinates_in_radius(center: SmallHexTile, radius: i32) -> Vec<SmallHexTile> {
        //Doesn't include center
        return HexCoordinates::coordinates_in_radius(center.into(), radius)
            .iter()
            .map(|a| SmallHexTile::from(a))
            .collect();
    }

    pub fn coordinates_in_radius_with_center_iter(center: SmallHexTile, radius: i32) -> impl Iterator<Item = SmallHexTile> {
        //DOES include center
        return HexCoordinates::coordinates_in_radius_with_center_iter(center.into(), radius).map(|a| SmallHexTile::from(a));
    }

    pub fn ring(center: SmallHexTile, radius: i32) -> Vec<SmallHexTile> {
        return HexCoordinates::ring(center.into(), radius)
            .iter()
            .map(|a| SmallHexTile::from(a))
            .collect();
    }

    pub fn ring_iter(center: SmallHexTile, radius: i32) -> impl Iterator<Item = SmallHexTile> {
        return HexCoordinates::ring_iter(center.into(), radius).map(|a| SmallHexTile::from(a));
    }

    pub fn shuffled_coordinates_between_radius(
        center: SmallHexTile,
        min_radius: i32,
        max_radius: i32,
        rng: &mut impl Rng,
    ) -> Vec<SmallHexTile> {
        if min_radius == 0 && max_radius == 0 {
            return Vec::new();
        }

        let mut candidate_tiles = SmallHexTile::coordinates_in_radius_with_center_iter(center, max_radius)
            .filter(|candidate| {
                let distance = center.distance_to(*candidate);
                distance >= min_radius && distance <= max_radius
            })
            .collect::<Vec<_>>();

        candidate_tiles.shuffle(rng);
        candidate_tiles
    }

    pub fn closest(&self, locations: &Vec<SmallHexTile>) -> Option<SmallHexTile> {
        let locations = locations.iter().map(|a| HexCoordinates::from(a)).collect();
        match HexCoordinates::from(self).closest(&locations) {
            Some(a) => return Some(SmallHexTile::from(a)),
            None => return None,
        }
    }

    pub fn is_center(&self) -> bool {
        return HexCoordinates::from(self).is_center();
    }

    pub fn to_center_position_xz(&self) -> Vector2 {
        return HexCoordinates::from(self).to_center_position_xz(false);
    }

    pub fn from_position(position: Vector2, dimension: u32) -> SmallHexTile {
        return SmallHexTile::from(HexCoordinates::from_position(position, false, dimension));
    }

    pub fn get_terrain_coordinates(&self) -> [LargeHexTile; 3] {
        let r = HexCoordinates::from(self).get_terrain_coordinates();
        return [r[0].into(), r[1].into(), r[2].into()];
    }

    //Fast, but not very precise
    pub fn simple_raycast<F: FnMut(SmallHexTile) -> bool>(start: &SmallHexTile, target: &SmallHexTile, mut is_valid_tile: F) -> bool {
        //https://www.redblobgames.com/grids/hexagons/#line-drawing
        //Skips start and target tiles
        let pstart = start.to_center_position_xz() + Vector2 { x: 0.01f32, y: 0.02f32 };
        let pend = target.to_center_position_xz();
        let n = start.distance_to(*target);
        for i in 1..n {
            let pcur = Vector2::lerp(&pstart, &pend, i as f32 / (n + 1) as f32);
            let cur = Self::from_position(pcur, start.dimension);
            if !is_valid_tile(cur) {
                return false;
            }
        }
        return true;
    }

    pub fn tile_count_in_radius(radius: i32) -> u64 {
        if radius < 0 {
            return 0;
        }

        let radius = radius as u64;
        1 + 3 * radius * (radius + 1)
    }

    pub fn tile_count_between_radius(min_radius: i32, max_radius: i32) -> u64 {
        Self::tile_count_in_radius(max_radius) - Self::tile_count_in_radius(min_radius - 1)
    }

    pub fn random_tile_between_radius(
        ctx: &ReducerContext,
        center: SmallHexTile,
        min_radius: i32,
        max_radius: i32,
        sampled: &mut HashSet<SmallHexTile>,
    ) -> SmallHexTile {
        loop {
            let candidate = SmallHexTile {
                x: center.x + ctx.rng().gen_range(-max_radius..=max_radius),
                z: center.z + ctx.rng().gen_range(-max_radius..=max_radius),
                dimension: center.dimension,
            };
            let distance = center.distance_to(candidate);
            if distance >= min_radius && distance <= max_radius && sampled.insert(candidate) {
                return candidate;
            }
        }
    }
}
