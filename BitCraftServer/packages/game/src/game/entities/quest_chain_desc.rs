use spacetimedb::ReducerContext;

use crate::{
    game::discovery::Discovery,
    messages::{
        components::{ExperienceState, InventoryState, PlayerState, QuestChainState, mobile_entity_state, quest_chain_state},
        static_data::{QuestChainDesc, QuestRequirement, QuestReward},
    },
    unwrap_or_err,
};

impl QuestChainDesc {
    pub fn give_rewards(&self, ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
        if self.rewards.is_empty() {
            return Ok(());
        }

        let mut discovery = Discovery::new(player_entity_id);

        for reward in self.rewards.iter() {
            match reward {
                QuestReward::PaddingNone(_) => {}

                QuestReward::ItemStack(stack) => {
                    let player_coord = ctx.db.mobile_entity_state().entity_id().find(player_entity_id).unwrap().coordinates();
                    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                        ctx,
                        player_entity_id,
                        &vec![*stack],
                        |x| x.distance_to(player_coord),
                        false,
                        || vec![player_coord],
                        true,
                    )?;
                }

                QuestReward::Achievement(_) => {}
                QuestReward::Collectible(_) => {}

                QuestReward::Experience(exp_stack) => {
                    ExperienceState::add_experience_f32(ctx, player_entity_id, exp_stack.skill_id, exp_stack.quantity);
                }

                QuestReward::SecondaryKnowledge(_) => {}
            }
        }

        discovery.commit(ctx);

        Ok(())
    }

    pub fn check_requirements(&self, ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
        if self.requirements.is_empty() {
            return Ok(());
        }

        let completed_chain_states: Vec<_> = ctx
            .db
            .quest_chain_state()
            .player_entity_id()
            .filter(&player_entity_id)
            .filter(|qcs: &QuestChainState| qcs.completed)
            .collect();

        for req in self.requirements.iter() {
            match req {
                QuestRequirement::PaddingNone(_) => {}

                QuestRequirement::QuestChain(required_desc_id) => {
                    if !completed_chain_states
                        .iter()
                        .any(|qcs: &QuestChainState| qcs.quest_chain_desc_id == *required_desc_id)
                    {
                        return Err("Quest chain requirements not met: Prerequisite chain incomplete.".into());
                    }
                }

                QuestRequirement::Achievement(_) => {}
                QuestRequirement::Collectible(_) => {}

                QuestRequirement::Level(required_level) => {
                    if !PlayerState::meets_level_requirement(ctx, player_entity_id, required_level) {
                        return Err("Quest chain requirements not met: Player level requirement not met.".into());
                    }
                }

                QuestRequirement::ItemStack(required_item_stack) => {
                    let inventory = unwrap_or_err!(
                        InventoryState::get_player_inventory(ctx, player_entity_id),
                        "Player has no inventory"
                    );
                    let stack = vec![*required_item_stack];
                    if !inventory.has(&stack) {
                        return Err("Quest chain requirements not met: Missing required items.".into());
                    }
                }
                QuestRequirement::SecondaryKnowledge(_) => {}
            }
        }

        Ok(())
    }
}
