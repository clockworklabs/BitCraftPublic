use spacetimedb::rand::Rng;
use spacetimedb::ReducerContext;
use spacetimedb::SpacetimeType;
use std::ops::Add;
use std::ops::Sub;

#[derive(SpacetimeType, PartialEq, PartialOrd, Eq, Clone, Copy, Debug)]
// #[sats(name = "HexDirection")]
// #[repr(i8)]
pub enum HexDirection {
    NE = 0, // Flat up right
    ENE,    // Pointy up right
    E,      // Flat right
    ESE,    // Pointy down right
    SE,     // Flat down right
    S,      // Pointy down
    SW,     // Flat down left
    WSW,    // Pointy down left
    W,      // Flat left
    WNW,    // Pointy up left
    NW,     // Flat up left
    N,      // Pointy up
}

impl From<i32> for HexDirection {
    fn from(int: i32) -> Self {
        match int {
            0 => Self::NE,  // Flat up right
            1 => Self::ENE, // Pointy up right
            2 => Self::E,   // Flat right
            3 => Self::ESE, // Pointy down right
            4 => Self::SE,  // Flat down right
            5 => Self::S,   // Pointy down
            6 => Self::SW,  // Flat down left
            7 => Self::WSW, // Pointy down left
            8 => Self::W,   // Flat left
            9 => Self::WNW, // Pointy up left
            10 => Self::NW, // Flat up left
            11 => Self::N,  // Pointy up
            _ => panic!("Invalid HexDirection {}", int),
        }
    }
}

impl Add for HexDirection {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let s = self as i32;
        let o = other as i32;
        let result = (s + o) % 12;
        HexDirection::from(result)
    }
}

impl Sub for HexDirection {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let s = self as i32;
        let o = other as i32;
        let result = (s - o).rem_euclid(12); // I believe equiv to ((a % b) + b) % b
        HexDirection::from(result)
    }
}

impl HexDirection {
    pub fn opposite(direction: HexDirection) -> HexDirection {
        if direction < HexDirection::from(6) {
            direction + HexDirection::from(6)
        } else {
            direction - HexDirection::from(6)
        }
    }

    pub fn previous(direction: HexDirection) -> HexDirection {
        direction - HexDirection::from(1)
    }

    pub fn previous_n(direction: HexDirection, n: i32) -> HexDirection {
        direction - HexDirection::from(n)
    }

    pub fn next(direction: HexDirection) -> HexDirection {
        direction + HexDirection::from(1)
    }

    pub fn next_n(direction: HexDirection, n: i32) -> HexDirection {
        direction + HexDirection::from(n)
    }

    pub fn previous_flat(direction: HexDirection) -> HexDirection {
        if direction == HexDirection::NE {
            HexDirection::NW
        } else {
            HexDirection::from(((direction as i32 >> 1) - 1) << 1)
        }
    }

    pub fn next_flat(direction: HexDirection) -> HexDirection {
        if direction == HexDirection::NW {
            HexDirection::NE
        } else {
            HexDirection::from(((direction as i32 >> 1) + 1) << 1)
        }
    }

    pub fn is_pointy(direction: HexDirection) -> bool {
        direction as i32 % 2 == 1
    }

    pub const POINTY: [HexDirection; 6] = [
        HexDirection::ENE,
        HexDirection::ESE,
        HexDirection::S,
        HexDirection::WSW,
        HexDirection::WNW,
        HexDirection::N,
    ];

    pub const FLAT: [HexDirection; 6] = [
        HexDirection::NE,
        HexDirection::E,
        HexDirection::SE,
        HexDirection::SW,
        HexDirection::W,
        HexDirection::NW,
    ];

    pub const ALL: [HexDirection; 12] = [
        HexDirection::NE,
        HexDirection::ENE,
        HexDirection::E,
        HexDirection::ESE,
        HexDirection::SE,
        HexDirection::S,
        HexDirection::SW,
        HexDirection::WSW,
        HexDirection::W,
        HexDirection::WNW,
        HexDirection::NW,
        HexDirection::N,
    ];

    pub fn to_str(&self) -> &'static str {
        match self {
            HexDirection::NE => "North East",
            HexDirection::ENE => "East North East",
            HexDirection::E => "East",
            HexDirection::ESE => "East South East",
            HexDirection::SE => "South East",
            HexDirection::S => "South",
            HexDirection::SW => "South West",
            HexDirection::WSW => "West South West",
            HexDirection::W => "West",
            HexDirection::WNW => "West North West",
            HexDirection::NW => "North West",
            HexDirection::N => "North",
        }
    }

    pub fn random(ctx: &ReducerContext) -> HexDirection {
        HexDirection::from(ctx.rng().gen_range(0..12))
    }

    pub fn radians_to_direction(angle: f64) -> HexDirection {
        let pi = std::f64::consts::PI;
        let bounds = [(pi * 1. / 12., pi * 3. / 12.),
            (pi * 3. / 12., pi * 5. / 12.),
            (pi * 5. / 12., pi * 7. / 12.), // 1.57
            (pi * 7. / 12., pi * 9. / 12.),
            (pi * 9. / 12., pi * 11. / 12.),
            (pi * 11. / 12., pi * 13. / 12.), // 3.1415 : Default case if < -11/12 PI
            (pi * -11. / 12., pi * -9. / 12.),
            (pi * -9. / 12., pi * -7. / 12.),
            (pi * -7. / 12., pi * -5. / 12.), // -1.57
            (pi * -5. / 12., pi * -3. / 12.),
            (pi * -3. / 12., -pi / 12.),
            (-pi / 12., pi * 1. / 12.)];
        for direction in HexDirection::ALL {
            if angle >= bounds[direction as usize].0 && angle < bounds[direction as usize].1 {
                return direction;
            }
        }
        HexDirection::S
    }

    pub fn direction_diff(d1: HexDirection, d2: HexDirection) -> i32 {
        let diff = (d1 as i32 - d2 as i32).abs();
        if diff <= 6 {
            diff
        } else {
            12 - diff
        }
    }
}
