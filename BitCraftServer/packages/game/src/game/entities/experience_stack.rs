pub use crate::messages::game_util::ExperienceStack;

impl ExperienceStack {
    // TODO: should these values be coming from static data?

    pub const MAX_LEVEL: i32 = 125;

    pub const MULTIPLIER: i32 = 10;

    pub const LEVEL_ONE_EXPERIENCE: i32 = 52;

    pub fn experience_for_level(level: i32) -> i32 {
        if level > Self::MAX_LEVEL {
            return -1;
        }

        let growth_rate = 2.0_f32.powf(0.158f32);

        f32::floor(
            Self::LEVEL_ONE_EXPERIENCE as f32
                * ((growth_rate.powf(level as f32) - growth_rate) / (growth_rate * growth_rate - growth_rate)),
        ) as i32
            * Self::MULTIPLIER
    }

    pub fn experience_until_next_level(level: i32) -> i32 {
        if level >= Self::MAX_LEVEL {
            return -1;
        }

        Self::experience_for_level(level + 1) - Self::experience_for_level(level)
    }

    pub fn level_for_experience(experience: i32) -> i32 {
        let mut level = 1;
        loop {
            if experience < Self::experience_for_level(level + 1) || level == Self::MAX_LEVEL {
                break;
            }
            level += 1;
        }
        level
    }
}
