use spacetimedb::{ReducerContext};

use crate::{messages::{components::{InventoryState, vault_state}, static_data::{CompletionCondition, QuestStageDesc}}, unwrap_or_err};

impl QuestStageDesc {
    pub fn fulfil_completion_conditions(&self, ctx: &ReducerContext, player_entity_id : u64) -> Result<(), String> {
        
        for cond in self.completion_conditions.iter() {
            match cond {
                CompletionCondition::PaddingNone(_) => {},
                CompletionCondition::ItemStack(c) => {
                    let inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, player_entity_id), "Player has no inventory");
                    let wallet = unwrap_or_err!(InventoryState::get_player_wallet(ctx, player_entity_id), "Player has no wallet");
                    let stack = vec![c.item_stack];
                    if !inventory.has(&stack) && !wallet.has(&stack){
                        return Err("Missing required items.".into());
                    }
                    if c.is_consumed {
                        InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, player_entity_id, &stack, |_| 0)?;
                    }
                }
                CompletionCondition::Achievement(_) => {},

                CompletionCondition::Collectible(collectible_id) => {
                    let vault_state = unwrap_or_err!(
                        ctx.db.vault_state()
                        .entity_id()
                        .find(&player_entity_id),
                        "No vault state for this player."
                    );
                    if !vault_state.has_collectible(*collectible_id) {
                        return Err("Missing required collectible.".into());
                    }
                },

                CompletionCondition::Level(_) => {},

                CompletionCondition::SecondaryKnowledge(_) => {},

                CompletionCondition::EquippedItem(_) => {},

            }
        }
        Ok(())
    }
}