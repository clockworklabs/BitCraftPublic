use spacetimedb::{ReducerContext, Table};

use crate::game::discovery::Discovery;
use crate::messages::components::{partial_experience_state, ExperienceState, PartialExperienceState};
use crate::messages::game_util::{ExperienceStack, ExperienceStackF32};
use crate::messages::static_data::{skill_desc, skill_level_knowledge_desc, AchievementDesc};
use crate::{experience_state, CharacterStatsState, SkillType};

impl ExperienceState {
    pub fn get_level(&self, skill_id: i32) -> i32 {
        let stack = if skill_id == SkillType::ANY as i32 {
            self.experience_stacks.iter().max_by_key(|s| s.quantity).unwrap()
        } else {
            self.experience_stacks.iter().find(|s| s.skill_id == skill_id).unwrap()
        };
        ExperienceStack::level_for_experience(stack.quantity)
    }

    pub fn add_experience(ctx: &ReducerContext, entity_id: u64, skill_id: i32, quantity: i32) {
        if quantity < 0 {
            panic!(
                "Invalid experience quantity: {}, entity id: {}, skill id: {}",
                quantity, entity_id, skill_id
            );
        }

        let mut experience_rate = CharacterStatsState::get_entity_stat(ctx, entity_id, crate::CharacterStatType::ExperienceRate);

        // If stat is zero, default to 100% (which is multiplier of 1.0)
        if experience_rate == 0f32 {
            experience_rate = 100f32;
        }

        let quantity_modified = (quantity as f32 * experience_rate / 100.0f32).round() as i32;
        Self::add_experience_internal(ctx, entity_id, skill_id, quantity_modified);
    }

    pub fn add_experience_f32(ctx: &ReducerContext, entity_id: u64, skill_id: i32, quantity: f32) {
        let mut experience_rate = CharacterStatsState::get_entity_stat(ctx, entity_id, crate::CharacterStatType::ExperienceRate);

        // If stat is zero, default to 100% (which is multiplier of 1.0)
        if experience_rate == 0f32 {
            experience_rate = 100f32;
        }

        let quantity_modified = quantity * (experience_rate / 100.0f32);

        let fraction = quantity_modified - quantity_modified.floor();
        let mut roll_over = 0;

        if fraction > 0.0 {
            if let Some(mut exp) = ctx.db.partial_experience_state().entity_id().find(&entity_id) {
                let mut found = false;
                for stack in &mut exp.experience_stacks {
                    if stack.skill_id == skill_id {
                        stack.quantity += fraction;
                        if stack.quantity >= 1.0 {
                            stack.quantity -= 1.0;
                            roll_over = 1;
                        }
                        found = true;
                    }
                }
                if !found {
                    exp.experience_stacks.push(ExperienceStackF32 {
                        skill_id,
                        quantity: fraction,
                    });
                }
                ctx.db.partial_experience_state().entity_id().update(exp);
            } else {
                let exp = PartialExperienceState {
                    entity_id,
                    experience_stacks: vec![ExperienceStackF32 {
                        skill_id,
                        quantity: fraction,
                    }],
                };
                ctx.db.partial_experience_state().insert(exp);
            }
        }

        let quantity = quantity_modified.floor() as i32 + roll_over;
        Self::add_experience_internal(ctx, entity_id, skill_id, quantity);
    }

    fn add_experience_internal(ctx: &ReducerContext, entity_id: u64, skill_id: i32, quantity: i32) {
        if let Some(mut exp) = ctx.db.experience_state().entity_id().find(&entity_id) {
            for stack in &mut exp.experience_stacks {
                if stack.skill_id == skill_id {
                    let old_level = ExperienceStack::level_for_experience(stack.quantity);

                    // prevent an unlikely i32 overrun
                    if stack.quantity > i32::MAX - quantity {
                        stack.quantity = i32::MAX;
                    } else {
                        stack.quantity += quantity;
                    }

                    //Cap XP level
                    let desc = ctx.db.skill_desc().id().find(skill_id).unwrap();
                    let max_xp = ExperienceStack::experience_for_level(desc.max_level);
                    stack.quantity = stack.quantity.min(max_xp);
                    let new_level = ExperienceStack::level_for_experience(stack.quantity);

                    ctx.db.experience_state().entity_id().update(exp);
                    Self::grant_skill_level_knowledge(ctx, entity_id, skill_id, old_level, new_level);
                    AchievementDesc::evaluate_all(entity_id);
                    break;
                }
            }
        }
    }

    pub fn grant_skill_level_knowledge(ctx: &ReducerContext, entity_id: u64, skill_id: i32, old_level: i32, new_level: i32) {
        if new_level <= old_level {
            return;
        }

        let mut discovery = Discovery::new(entity_id);
        for level in old_level + 1..=new_level {
            for reward in ctx.db.skill_level_knowledge_desc().skill_level().filter((skill_id, level)) {
                discovery.acquire_secondary(ctx, reward.secondary_knowledge_id);
            }
        }
        discovery.commit(ctx);
    }
}
