pub const RADIUS_RATIO: f32 = 0.866025404;
pub const TERRAIN_OUTER_RADIUS: f32 = 10.0;
pub const TERRAIN_INNER_RADIUS: f32 = TERRAIN_OUTER_RADIUS * RADIUS_RATIO;
pub const OUTER_RADIUS: f32 = TERRAIN_OUTER_RADIUS / 3.0;
pub const INNER_RADIUS: f32 = OUTER_RADIUS * RADIUS_RATIO;
pub const SOLID_FACTOR: f32 = 0.95;
pub const SEPARATOR_DEPTH_FACTOR: f32 = 0.5;
pub const BLEND_FACTOR: f32 = 1.0 - SOLID_FACTOR;
pub const CELL_PERTURB_STRENGTH: f32 = 4.0;
pub const NUM_NEIGHBORS: f32 = 6.0;
pub const ELEVATION_STEP: f32 = 1.0;
pub const WATER_ELEVATION_OFFSET: f32 = -0.5 * ELEVATION_STEP;
pub const WATER_FACTOR: f32 = 0.6;
pub const WATER_BLEND_FACTOR: f32 = 1.0 - WATER_FACTOR;
pub const BRIDGE_WIDTH: f32 = (1.0 - SOLID_FACTOR) * TERRAIN_OUTER_RADIUS;
pub const HASH_GRID_SIZE: f32 = 256.0;
pub const HASH_GRID_SCALE: f32 = 0.25;

pub const FLOAT_COORD_PRECISION: u32 = 3;
pub const FLOAT_COORD_PRECISION_MUL: i32 = 10i32.pow(FLOAT_COORD_PRECISION);

// Returned when comparing coordinates across DIFFERENT dimensions.
// Represents an unreachable distance rather than a real geometric value.
// Use MAX / 10 instead of MAX to avoid integer overflows.
pub const UNREACHABLE_DISTANCE_I32: i32 = i32::MAX / 10;
pub const UNREACHABLE_DISTANCE_F32: f32 = f32::INFINITY;