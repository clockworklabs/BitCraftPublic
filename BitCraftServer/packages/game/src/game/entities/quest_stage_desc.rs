use spacetimedb::{ReducerContext};

use crate::{game::{discovery::Discovery, game_state::game_state_filters}, messages::{components::{InventoryState, vault_state}, static_data::{CompletionCondition, QuestStageDesc}}, unwrap_or_err};

impl QuestStageDesc {
    pub fn fulfil_completion_conditions(&self, ctx: &ReducerContext, player_entity_id : u64) -> Result<(), String> {
        
        for cond in self.completion_conditions.iter() {
            match cond {
                CompletionCondition::PaddingNone(_) => {},
                CompletionCondition::ItemStack(c) => {
                    let stack = vec![c.item_stack];
                    let coord = game_state_filters::coordinates_float(ctx, player_entity_id).parent_small_tile();

                    if !InventoryState::has_items_in_player_inventory_and_nearby_deployables(ctx, player_entity_id, &stack, |c| c.distance_to(coord))? {
                        return Err("Missing required items.".into());
                    }

                    if c.is_consumed {
                        InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, player_entity_id, &stack, |c| c.distance_to(coord))?;
                    }
                }
                CompletionCondition::Achievement(_) => {},

                CompletionCondition::Collectible(collectible_id) => {
                    let vault_state = unwrap_or_err!(
                        ctx.db.vault_state()
                        .entity_id()
                        .find(player_entity_id),
                        "No vault state for this player."
                    );
                    if !vault_state.has_collectible(*collectible_id) {
                        return Err("Missing required collectible.".into());
                    }
                },

                CompletionCondition::Level(_) => {},

                CompletionCondition::SecondaryKnowledge(knowledge_id) => {
                    if !Discovery::already_acquired_secondary(ctx, player_entity_id, *knowledge_id) {
                        return Err("Missing required knowledge".into());
                    }
                },

                CompletionCondition::EquippedItem(_) => {},

            }
        }
        Ok(())
    }
}