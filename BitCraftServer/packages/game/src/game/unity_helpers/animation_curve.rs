use crate::messages::world_gen::{WorldGenAnimationCurve, WorldGenAnimationCurveKeyframe};

pub type Keyframe = WorldGenAnimationCurveKeyframe;
pub type AnimationCurve = WorldGenAnimationCurve;

impl Keyframe {
    pub fn new_proto(val: WorldGenAnimationCurveKeyframe) -> Self {
        Self {
            time: val.time,
            value: val.value,
            in_tangent: val.in_tangent,
            out_tangent: val.out_tangent,
        }
    }
}

impl AnimationCurve {
    //https://answers.unity.com/questions/464782/t-is-the-math-behind-animationcurveevaluate.html (SuperPingu + DaNerdyDude + optimization)
    pub fn evaluate(&self, t: f32) -> f32 {
        let (k0, k1) = self.get_keyframes_for(t);

        let p1x = k0.time;
        let p1y = k0.value;
        let tp1 = k0.out_tangent;
        let p2x = k1.time;
        let p2y = k1.value;
        let tp2 = k1.in_tangent;

        let p1x2 = p1x * p1x;
        let p1x3 = p1x2 * p1x;
        let p2x2 = p2x * p2x;
        let p2x3 = p2x2 * p2x;
        let p1xp2x = p1x * p2x;
        let p2ymp1y = p2y - p1y;

        let divisor = p1x3 - p2x3 + (3.0 * p1xp2x * (p2x - p1x));
        let a = ((tp1 + tp2) * (p1x - p2x) + p2ymp1y * 2.0) / divisor;
        let b =
            (2.0 * (p2x2 * tp1 - p1x2 * tp2) - p1x2 * tp1 + p2x2 * tp2 + p1xp2x * (tp2 - tp1) + 3.0 * (p1x + p2x) * (p1y - p2y)) / divisor;
        let c = (p1x3 * tp2 - p2x3 * tp1 + p1xp2x * (p1x * (2.0 * tp1 + tp2) - p2x * (tp1 + 2.0 * tp2)) + 6.0 * p1xp2x * p2ymp1y) / divisor;
        let d = ((p1x * p2x2 - p1x2 * p2x) * (p2x * tp1 + p1x * tp2) - p1y * p2x3 + p1x3 * p2y + 3.0 * p1xp2x * (p2x * p1y - p1x * p2y))
            / divisor;

        let t2 = t * t;
        a * t2 * t + b * t2 + c * t + d
    }

    fn get_keyframes_for(&self, t: f32) -> (&Keyframe, &Keyframe) {
        for (i, k) in self.keyframes.iter().enumerate() {
            if k.time > t {
                return (&self.keyframes[i - 1 ], k);
            }
        }

        let l = self.keyframes.len();
        (&self.keyframes[l - 2], &self.keyframes[l - 1])
    }

    pub fn ease_in_out(start_time: f32, start_val: f32, ent_time: f32, end_val: f32) -> Self {
        Self {
            keyframes: vec![
                Keyframe {
                    time: start_time,
                    value: start_val,
                    in_tangent: 0f32,
                    out_tangent: 0f32,
                },
                Keyframe {
                    time: ent_time,
                    value: end_val,
                    in_tangent: 0f32,
                    out_tangent: 0f32,
                },
            ],
        }
    }

    pub fn linear(start_time: f32, start_val: f32, ent_time: f32, end_val: f32) -> Self {
        Self {
            keyframes: vec![
                Keyframe {
                    time: start_time,
                    value: start_val,
                    in_tangent: 0f32,
                    out_tangent: 1f32,
                },
                Keyframe {
                    time: ent_time,
                    value: end_val,
                    in_tangent: 1f32,
                    out_tangent: 0f32,
                },
            ],
        }
    }
}
