use crate::{
    game::{coordinates::*, world_gen::open_simplex_noise::OpenSimplex},
    messages::static_data::{wind_dbg_desc, wind_params_desc},
};
use lazy_static::lazy_static;
use spacetimedb::{ReducerContext, Table};

//Consts must match client values
const SEED: i64 = 1596423;
const LOOP_RADIUS: f64 = 100_000.0;
const NOISE_MUL: f32 = 10.0; //Noise rarely reaches min/max values, so we multiply it to increase likelyhood of those values
const TIMESTAMP_OFFSET: u64 = 1750000000000; //Make number smaller to minimize floating point errors

lazy_static! {
    pub static ref NOISE: OpenSimplex = OpenSimplex::from_seed(SEED);
}

///Returns direction in radians [0..2PI]
pub fn sample_wind_float(ctx: &ReducerContext, coord: &FloatHexTile, time: Option<u64>) -> f32 {
    let pos = coord.to_world_position();
    let time = time.unwrap_or_else(|| super::unix_ms(ctx.timestamp));
    sample_wind_vec2(ctx, pos.x as f64, pos.y as f64, time)
}

///Returns direction in radians [0..2PI]
pub fn sample_wind_small(ctx: &ReducerContext, coord: &SmallHexTile, time: Option<u64>) -> f32 {
    let pos = coord.to_center_position_xz();
    let time = time.unwrap_or_else(|| super::unix_ms(ctx.timestamp));
    sample_wind_vec2(ctx, pos.x as f64, pos.y as f64, time)
}

///Returns direction in radians [0..2PI]
pub fn sample_wind_large(ctx: &ReducerContext, coord: &LargeHexTile, time: Option<u64>) -> f32 {
    let pos = coord.to_center_position_xz();
    let time = time.unwrap_or_else(|| super::unix_ms(ctx.timestamp));
    sample_wind_vec2(ctx, pos.x as f64, pos.y as f64, time)
}

///Returns direction in radians [0..2PI]
pub fn sample_wind_vec2(ctx: &ReducerContext, x: f64, y: f64, time_ms: u64) -> f32 {
    let time_ms = time_ms - TIMESTAMP_OFFSET; //Make number smaller to minimize floating point errors
    let mut sum = 0.0;
    let mut div = 0.0;
    let time_multiplier = match ctx.db.wind_dbg_desc().id().find(0) {
        Some(r) => r.time_multiplier,
        None => 1.0,
    };
    for wind in ctx.db.wind_params_desc().iter() {
        let phase = time_ms as f64 / wind.cycle_sec as f64 / 1000.0 * 2.0 * std::f64::consts::PI * time_multiplier;
        let z = phase.sin() * LOOP_RADIUS;
        let w = phase.cos() * LOOP_RADIUS;
        sum += NOISE.evaluate4(x * wind.scale, y * wind.scale, z, w) * wind.weight;
        div += wind.weight;
    }
    if div == 0.0 {
        return 0.0;
    }
    let val = (sum / div) as f32 + 1.0; //[0..2]
    let val = (val * NOISE_MUL * std::f32::consts::PI * 2.0) % (std::f32::consts::PI * 2.0); //Noise rarely reaches min/max values, so we multiply it to increase likelyhood of those values
    val
}
