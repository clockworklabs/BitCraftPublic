use crate::messages::world_gen::WorldGenVector2;

pub type Vector2 = WorldGenVector2;

impl std::ops::Add<&Vector2> for Vector2 {
    type Output = Vector2;

    fn add(self, other: &Vector2) -> Vector2 {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Add<Vector2> for Vector2 {
    type Output = Vector2;

    fn add(self, other: Vector2) -> Vector2 {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub<&Vector2> for Vector2 {
    type Output = Vector2;

    fn sub(self, other: &Vector2) -> Vector2 {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Sub<Vector2> for Vector2 {
    type Output = Vector2;

    fn sub(self, other: Vector2) -> Vector2 {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<f32> for Vector2 {
    type Output = Vector2;

    fn mul(self, other: f32) -> Vector2 {
        Vector2 {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

impl std::ops::Div<f32> for Vector2 {
    type Output = Vector2;

    fn div(self, other: f32) -> Vector2 {
        Vector2 {
            x: self.x / other,
            y: self.y / other,
        }
    }
}

impl std::marker::Copy for Vector2 {}

impl Vector2 {
    pub fn negative_infinity() -> Self {
        Self {
            x: f32::NEG_INFINITY,
            y: f32::NEG_INFINITY,
        }
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn sqr_magnitude(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn dot(a: &Vector2, b: &Vector2) -> f32 {
        a.x * b.x + a.y * b.y
    }

    pub fn lerp(a: &Vector2, b: &Vector2, t: f32) -> Vector2 {
        *a + (*b - *a) * t.clamp(0.0, 1.0)
    }

    pub fn normalized(self) -> Vector2 {
        let mag = self.magnitude();
        self / mag
    }
}
