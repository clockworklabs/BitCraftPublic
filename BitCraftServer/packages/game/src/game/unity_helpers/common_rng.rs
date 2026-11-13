#![allow(dead_code)]

use std::num::Wrapping;

use spacetimedb::SpacetimeType;

//RNG with shared implementation between Unity and Rust
//https://en.wikipedia.org/wiki/Xorshift    (xorshift+)
#[derive(Debug, Copy, Clone, SpacetimeType)]
pub struct CommonRNG {
    seed: i32,
    x: u64,
    y: u64,
}

impl CommonRNG {
    pub fn from_seed(seed: i32) -> Self {
        let mut rng = Self { seed, x: 0, y: 0 };
        rng.reset();
        rng
    }

    pub fn reset(&mut self) {
        unsafe {
            self.x = *(&self.seed as *const i32 as *const u32) as u64;
            self.y = *(&self.seed as *const i32 as *const u32) as u64 ^ 8943154768516357321;
        }
    }

    #[expect(clippy::should_implement_trait, reason = "port of the Unity CommonRNG API; an infinite RNG step is not an Iterator")]
    pub fn next(&mut self) -> u64 {
        let mut t = Wrapping(self.x);
        let s = Wrapping(self.y);
        self.x = s.0;
        t ^= t << 23;
        t ^= t >> 18;
        t ^= s ^ (s >> 5);
        self.y = t.0;
        (t + s).0
    }

    //https://stackoverflow.com/questions/19167844/proper-way-to-generate-a-random-float-given-a-binary-random-number-generator
    pub fn next_double_01(&mut self) -> f64 {
        unsafe {
            let mut val = self.next();
            val = (1023 << 52) | (val & 0xfffffffffffff);
            *(&val as *const u64 as *const f64) - 1.0
        }
    }

    pub fn i32_range(&mut self, min: i32, max: i32) -> i32 {
        ((self.next() % (Wrapping(max) - Wrapping(min)).0 as u64) as i64 + min as i64) as i32
    }

    pub fn f32_range(&mut self, min: f32, max: f32) -> f32 {
        (self.next_double_01() * (max - min) as f64 + min as f64) as f32
    }

    pub fn usize_range(&mut self, min: usize, max: usize) -> usize {
        ((self.next() % (Wrapping(max) - Wrapping(min)).0 as u64) as i64 + min as i64) as usize
    }

    pub fn i32(&mut self, min: i32, max: i32) -> i32 {
        self.i32_range(min, max)
    }

    pub fn f32(&mut self, min: f32, max: f32) -> f32 {
        self.f32_range(min, max)
    }

    pub fn bool(&mut self, chance: f32) -> bool {
        self.next_double_01() < chance as f64
    }

    pub fn usize(&mut self, min: usize, max: usize) -> usize {
        self.usize_range(min, max)
    }
}
