use std::collections::HashMap;

use spacetimedb::{rand::Rng, ReducerContext};

use crate::{
    item_desc, item_list_desc,
    messages::{
        game_util::{ItemStack, ItemType},
        static_data::{ItemDesc, ItemListDesc, QuestDropDesc},
    },
};

impl ItemListDesc {
    /*
    pub fn convert_item_stacks(item_stacks: Vec<ItemStack>) -> Vec<ItemStack> {
        let mut resulting_item_stacks = Vec::new();

        for item_stack in item_stacks.iter() {
            let mut new_item_stacks = Self::convert_item_stack(*item_stack);
            resulting_item_stacks.append(&mut new_item_stacks);
        }
        resulting_item_stacks
    }

    pub fn convert_item_stack(item_stack: ItemStack) -> Vec<ItemStack> {
        if item_stack.item_type == ItemType::ItemList {
            Self::extract_item_stacks(item_stack.item_id)
        } else {
            vec![item_stack]
        }
    }
    */

    pub fn extract_item_stacks_from_item(ctx: &ReducerContext, item_stack: ItemStack, extracting_player_id: Option<u64>) -> Vec<ItemStack> {
        if item_stack.item_type == ItemType::Cargo {
            return vec![item_stack];
        }

        let item_list_id = ctx.db.item_desc().id().find(item_stack.item_id).unwrap().item_list_id;
        if item_list_id == 0 {
            return vec![item_stack];
        }

        Self::extract_item_stacks_multiple(ctx, item_list_id, item_stack.quantity, extracting_player_id)
    }

    pub fn extract_item_stacks(ctx: &ReducerContext, item_list_id: i32, extracting_player_id: Option<u64>) -> Vec<ItemStack> {
        Self::extract_item_stacks_multiple(ctx, item_list_id, 1, extracting_player_id)
    }

    pub fn extract_item_stacks_multiple(ctx: &ReducerContext, item_list_id: i32, rolls: i32, extracting_player_id: Option<u64>) -> Vec<ItemStack> {
        let item_list = ctx.db.item_list_desc().id().find(item_list_id).unwrap();
        let item_stacks: Vec<ItemStack> = Vec::new();
        item_list.roll(ctx, rolls, item_stacks, extracting_player_id)
    }

    pub fn roll(&self, ctx: &ReducerContext, num_rolls: i32, mut item_stacks: Vec<ItemStack>, rolling_player_id: Option<u64>) -> Vec<ItemStack> {
        let mut item_desc_cache: HashMap<i32, ItemDesc> = HashMap::new();

        let mut sum = 0.0;
        for p in &self.possibilities {
            sum += p.probability;
        }

        for _i in 0..num_rolls {
            let mut rnd = ctx.rng().gen_range(0.0..=sum);
            for p in &self.possibilities {
                rnd -= p.probability;
                if rnd <= 0.0 {
                    // Selected this entry
                    for item_stack in p.items.iter() {
                        if item_stack.item_type != ItemType::Item {
                            item_stacks.push(*item_stack);
                            continue;
                        }

                        let item_desc = item_desc_cache
                            .entry(item_stack.item_id)
                            .or_insert(ctx.db.item_desc().id().find(item_stack.item_id).unwrap());

                        if item_desc.item_list_id == 0 {
                            item_stacks.push(*item_stack);
                            continue;
                        }

                        // ItemList containing itemlists might not be efficient for high amount of rolls
                        item_stacks.extend(Self::extract_item_stacks_from_item(ctx, *item_stack, rolling_player_id));
                    }
                    break;
                }
            }
        }

        // Quest Drops
        if let Some(player_id) = rolling_player_id {
            if player_id != 0 {
                if let Some(mut drops) = QuestDropDesc::roll_item_list(ctx, player_id, self.id, num_rolls) {
                    item_stacks.append(&mut drops);
                }
            }
        }

        item_stacks
    }
}
