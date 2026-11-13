pub mod f32 {
    pub fn half_to_even(f: f32) -> f32 {
        let r = f.round();
        let d = r - f;

        if (d != 0.5f32) && (d != -0.5f32) {
            return r;
        }

        if r % 2.0f32 == 0.0f32 {
            return r;
        }

        f - d
    }

    pub fn map(value: f32, x1: f32, x2: f32, y1: f32, y2: f32) -> f32 {
        let m = (y2 - y1) / (x2 - x1);
        let c = y1 - m * x1;

        m * value + c
    }

    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        (b - a) * t + a
    }

    pub fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
        (value - a) / (b - a)
    }
}

pub mod f64 {
    pub fn half_to_even(f: f64) -> f64 {
        let r = f.round();
        let d = r - f;

        if (d != 0.5f64) && (d != -0.5f64) {
            return r;
        }

        if r % 2.0f64 == 0.0f64 {
            return r;
        }

        f - d
    }
}
