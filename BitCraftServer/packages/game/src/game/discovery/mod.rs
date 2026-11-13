use std::collections::HashSet;

use spacetimedb::ReducerContext;

use crate::messages::components::*;
use crate::messages::game_util::ItemStack;
use crate::messages::game_util::ItemType;

pub mod autogen;
mod discovery_setters;

use self::autogen::_discovery::Knowledges;

pub struct Discovery {
    pub player_entity_id: u64,
    pub knowledges: Option<Knowledges>,
    pub acquired_achievement: bool,
}

impl Discovery {
    pub fn new(player_entity_id: u64) -> Discovery {
        Discovery {
            player_entity_id,
            knowledges: None,
            acquired_achievement: false,
        }
    }

    pub fn has_player_acquired_lore(ctx: &ReducerContext, player_entity_id: u64, lore_id: i32) -> bool {
        if let Some(knowledge) = ctx.db.knowledge_lore_state().entity_id().find(player_entity_id) {
            if let Some(entry) = knowledge.entries.iter().find(|e| e.id == lore_id) {
                return entry.state == KnowledgeState::Acquired;
            }
        }
        false
    }

    pub fn discover_item_stack(&mut self, ctx: &ReducerContext, item_stack: &ItemStack) {
        if item_stack.item_type == ItemType::Cargo {
            self.discover_cargo(ctx, item_stack.item_id);
        } else {
            self.discover_item_and_item_list(ctx, item_stack.item_id);
        }
    }

    pub fn acquire_item_stack(&mut self, ctx: &ReducerContext, item_stack: &ItemStack) {
        if item_stack.item_type == ItemType::Cargo {
            self.acquire_cargo(ctx, item_stack.item_id);
        } else {
            self.discover_item_and_item_list(ctx, item_stack.item_id);
            self.acquire_item(ctx, item_stack.item_id);
        }
    }

    pub fn acquire_item_stacks(&mut self, ctx: &ReducerContext, item_stacks: &Vec<ItemStack>) {
        let mut seen = HashSet::new();
        for item_stack in item_stacks {
            if seen.insert((item_stack.item_id, item_stack.item_type as i32)) {
                self.acquire_item_stack(ctx, item_stack);
            }
        }
    }
}
