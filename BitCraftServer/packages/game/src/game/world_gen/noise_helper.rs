use std::sync::Mutex;

use num_traits::pow;

use crate::game::unity_helpers::common_rng::CommonRNG;

use super::{super::unity_helpers::vector2::Vector2, open_simplex_noise::OpenSimplex};

//Returns a 2d vector of floats range 0..1 inc
#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
pub fn get_map(
    width: i32,
    depth: i32,
    seed: i32,
    scale: f32,
    octaves: i32,
    persistance: f32,
    lacunarity: f32,
    offset: Vector2,
) -> Vec<Vec<f32>> {
    let mut map = vec![vec![0.0; depth as usize]; width as usize];

    let mut random = CommonRNG::from_seed(seed);
    let mut offsets: Vec<Vector2> = Vec::with_capacity(octaves as usize);
    for _ in 0..octaves {
        let x = random.f32(-100000.0, 100000.0) + offset.x;
        let y = random.f32(-100000.0, 100000.0) + offset.y;
        let off = Vector2 { x, y };
        offsets.push(off);
    }

    let simplex = OpenSimplex::from_seed(seed as i64);

    let h_width = width as f32 / 2.0;
    let h_depth = depth as f32 / 2.0;

    let mut min: f32 = f32::MAX;
    let mut max: f32 = f32::MIN;
    for x in 0..width {
        for y in 0..depth {
            let mut amplitude = 1.0f32;
            let mut frequency = 1.0f32;
            let mut height = 0.0f32;

            for off in &offsets {
                let sample = Vector2 {
                    x: x as f32 - h_width,
                    y: y as f32 - h_depth,
                } * (scale * frequency)
                    + off;

                let value = simplex.evaluate(sample.x as f64, sample.y as f64) as f32 * 2.0 - 1.0;
                height += value * amplitude;

                amplitude *= persistance;
                frequency *= lacunarity;
            }

            min = min.min(height);
            max = max.max(height);

            map[x as usize][y as usize] = height;
        }
    }

    for x in 0..width {
        for y in 0..depth {
            map[x as usize][y as usize] = inverse_lerp(min, max, map[x as usize][y as usize]);
        }
    }

    map
}

pub fn get(position: Vector2, scale: f32, octaves: i32, persistance: f32, lacunarity: f32, offset: Vector2) -> f32 {
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut height = 0.0f32;

    let mut max_amplitude = 0.0f32;
    for i in 0..octaves {
        max_amplitude += pow(persistance, i as usize);
    }

    for i in 0..octaves {
        let offset = get_octave_offset(i as usize) + offset;
        let sample = position * scale * frequency + offset;

        let value = (get_simplex(sample.x, sample.y) + 1f32) * 0.5f32;
        height += value * amplitude;

        amplitude *= persistance;
        frequency *= lacunarity;
    }

    height /= max_amplitude;

    height
}

fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    (value - a) / (b - a)
}

thread_local! {
    static RANDOM: Mutex<CommonRNG> = Mutex::new(CommonRNG::from_seed(329817290));
}

thread_local! {
    static SIMPLEX: Mutex<OpenSimplex> =
        Mutex::new(OpenSimplex::from_seed(3298172903027438190));
}

thread_local! {
    static OFFSETS: Mutex<Vec<Vector2>> = const { Mutex::new(vec![]) };
}

fn random() -> f32 {
    let mut random_value: f32 = 0.0f32;
    RANDOM.with(|random| {
        let mut random = random.lock().unwrap();
        random_value = random.f32(-100000f32, 100000f32);
    });
    random_value
}

fn get_simplex(x: f32, y: f32) -> f32 {
    let mut simplex_value: f32 = 0.0f32;
    SIMPLEX.with(|simplex| {
        let simplex = simplex.lock().unwrap();
        simplex_value = simplex.evaluate(x as f64, y as f64) as f32
    });

    simplex_value
}

fn get_octave_offset(index: usize) -> Vector2 {
    let mut vector2_value: Vector2 = Vector2::new(0f32, 0f32);
    OFFSETS.with(|offsets| {
        let mut offsets = offsets.lock().unwrap();

        for _ in offsets.len()..(index + 1) {
            let x = random();
            let y = random();

            offsets.push(Vector2 { x, y });
        }

        vector2_value = offsets[index];
    });

    vector2_value
}
