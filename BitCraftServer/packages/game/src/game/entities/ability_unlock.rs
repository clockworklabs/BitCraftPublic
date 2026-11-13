use spacetimedb::ReducerContext;

use crate::messages::{
    components::*,
    static_data::{ability_unlock_desc, AbilityUnlockDesc},
};

impl AbilityUnlockDesc {
    pub fn evaluate(ctx: &ReducerContext, actor_id: u64, ability_enum: AbilityTypeEnum, ability_data: AbilityType) -> Result<(), String> {
        for current in ctx.db.ability_unlock_desc().ability_type_enum().filter(ability_enum as i32) {
            let matching = if let Some(a) = current.ability_data {
                a == ability_data
            } else {
                true
            };
            if matching {
                // evaluate
                let secondary_knowledge = ctx.db.knowledge_secondary_state().entity_id().find(actor_id).unwrap();

                for knowledge_id in &current.blocking_knowledges {
                    if secondary_knowledge
                        .entries
                        .iter()
                        .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired)
                    {
                        return Err("This ability is no longer available to you".into());
                    }
                }

                // Validate that the player belongs to a claim having unlocked the required claim tech
                if current.required_claim_tech_id != 0 {
                    let mut found_tech = false;
                    for claim_member in ctx.db.claim_member_state().player_entity_id().filter(actor_id) {
                        // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
                        if let Some(claim_tech) = ctx.db.claim_tech_state().entity_id().find(claim_member.claim_entity_id) {
                            found_tech |= claim_tech.has_unlocked_tech(current.required_claim_tech_id);
                        }
                    }
                    if !found_tech {
                        return Err("You are missing the claim tech to perform this ability".into());
                    }
                }

                for required_knowledge_id in &current.required_knowledges {
                    if secondary_knowledge
                        .entries
                        .iter()
                        .find(|knowledge| knowledge.id == *required_knowledge_id && knowledge.state == KnowledgeState::Acquired)
                        .is_none()
                    {
                        return Err("You don't have the knowledge required to perform this ability".into());
                    }
                }

                //check player level
                for level_requirement in current.level_requirements {
                    let player_level = ctx
                        .db
                        .experience_state()
                        .entity_id()
                        .find(actor_id)
                        .unwrap()
                        .get_level(level_requirement.skill_id);
                    if player_level < level_requirement.level {
                        return Err("Your skill level is too low to perform this ability.".into());
                    }
                }

                // We assume there can be only one match per ability
                return Ok(());
            }
        }
        // This ability is not gated
        Ok(())
    }
}
