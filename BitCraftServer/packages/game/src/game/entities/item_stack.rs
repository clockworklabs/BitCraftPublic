use spacetimedb::ReducerContext;

use crate::{
    collectible_desc,
    game::{discovery::Discovery, entities::food},
    item_desc, knowledge_scroll_desc,
    messages::{
        game_util::{InputItemStack, ItemStack, ItemType},
        static_data::{food_desc, ItemListDesc},
    },
    vault_state, TradeOrderState,
};

impl Copy for ItemStack {}

impl ItemType {
    pub fn to_enum(value: i32) -> ItemType {
        unsafe { std::mem::transmute(value) }
    }
}

impl ItemStack {
    pub fn new(ctx: &ReducerContext, item_id: i32, item_type: ItemType, quantity: i32) -> ItemStack {
        let durability = if item_type == ItemType::Item {
            let d = ctx.db.item_desc().id().find(&item_id).unwrap().durability;
            if d <= 0 {
                None
            } else {
                Some(d)
            }
        } else {
            None
        };
        ItemStack {
            item_id,
            item_type,
            quantity,
            durability,
        }
    }

    pub fn from_input(input_item_stack: &InputItemStack) -> Self {
        Self::new_ignore_durability(input_item_stack.item_id, input_item_stack.item_type, input_item_stack.quantity)
    }

    pub fn new_ignore_durability(item_id: i32, item_type: ItemType, quantity: i32) -> ItemStack {
        ItemStack {
            item_id,
            item_type,
            quantity,
            durability: None,
        }
    }

    pub fn hex_coins(quantity: i32) -> ItemStack {
        ItemStack {
            item_id: TradeOrderState::MARKET_MODE_CURRENCY_ID,
            item_type: ItemType::Item,
            quantity,
            durability: None,
        }
    }

    pub fn empire_currency(ctx: &ReducerContext, quantity: i32) -> ItemStack {
        ItemStack {
            item_id: ctx.db.item_desc().tag().filter("Empire Currency").next().unwrap().id,
            item_type: ItemType::Item,
            quantity,
            durability: None,
        }
    }

    pub fn single_cargo(item_id: i32) -> ItemStack {
        ItemStack {
            item_id,
            item_type: ItemType::Cargo,
            quantity: 1,
            durability: None,
        }
    }

    pub fn hexite_capsule() -> ItemStack {
        Self::single_cargo(2000000 /*Hexite Capsule*/)
    }

    pub fn clone_with_quantity(&self, quantity: i32) -> Self {
        let mut item_stack = self.clone();
        item_stack.quantity = quantity;
        item_stack
    }

    pub fn empty() -> ItemStack {
        ItemStack {
            item_id: 0,
            item_type: ItemType::Item,
            quantity: 0,
            durability: None,
        }
    }

    pub fn merge(&mut self, other: ItemStack) -> bool {
        if self.item_id != other.item_id || self.item_type != other.item_type {
            return false;
        }
        self.quantity += other.quantity;
        true
    }

    pub fn merge_multiple(item_stacks: &Vec<ItemStack>) -> Vec<ItemStack> {
        let mut merged_stacks: Vec<ItemStack> = Vec::new();
        for stack in item_stacks.clone() {
            let merged_index = merged_stacks
                .iter()
                .position(|&st| st.item_id == stack.item_id && st.item_type == stack.item_type);
            match merged_index {
                Some(i) => {
                    merged_stacks[i].merge(stack);
                }
                None => merged_stacks.push(stack),
            }
        }
        merged_stacks
    }

    pub fn is_auto_collect(&self, ctx: &ReducerContext) -> bool {
        if self.item_type == ItemType::Item {
            // parse and remove all items that are automatically turned into collectibles
            if let Some(collectible) = ctx.db.collectible_desc().item_deed_id().filter(self.item_id).next() {
                if collectible.auto_collect {
                    return true;
                }
            } else if let Some(scroll) = ctx.db.knowledge_scroll_desc().item_id().find(&self.item_id) {
                return scroll.auto_collect;
            }
        }
        false
    }

    pub fn can_auto_collect(&mut self, ctx: &ReducerContext, player_entity_id: u64) -> (bool, bool) {
        // (Is auto-collectable / would auto-collect)

        if self.item_type == ItemType::Item && player_entity_id != 0 {
            // parse and remove all items that are automatically turned into collectibles
            if let Some(collectible) = ctx.db.collectible_desc().item_deed_id().filter(self.item_id).next() {
                if collectible.auto_collect {
                    let vault_state = ctx.db.vault_state().entity_id().find(&player_entity_id).unwrap();
                    return (true, vault_state.collectibles.iter().find(|c| c.id == collectible.id).is_some());
                }
            } else if let Some(scroll) = ctx.db.knowledge_scroll_desc().item_id().find(&self.item_id) {
                if scroll.auto_collect {
                    if scroll.secondary_knowledge_id != 0 {
                        return (
                            true,
                            Discovery::already_acquired_secondary(ctx, player_entity_id, scroll.secondary_knowledge_id),
                        );
                    } else {
                        return (true, Discovery::already_acquired_lore(ctx, player_entity_id, scroll.item_id));
                    }
                }
            }
        }
        (false, false)
    }

    pub fn auto_collect(&mut self, ctx: &ReducerContext, discovery: &mut Discovery, player_entity_id: u64) -> bool {
        if self.item_type == ItemType::Item && player_entity_id != 0 {
            // parse and remove all items that are automatically turned into collectibles
            if let Some(collectible) = ctx.db.collectible_desc().item_deed_id().filter(self.item_id).next() {
                if collectible.auto_collect {
                    let mut vault_state = ctx.db.vault_state().entity_id().find(&player_entity_id).unwrap();
                    let _ = vault_state.add_collectible(ctx, collectible.id, false);
                    ctx.db.vault_state().entity_id().update(vault_state);
                    self.quantity = 0; // this will consume all instances of this item but those stacks should only have 1 anyways
                }
            } else if let Some(scroll) = ctx.db.knowledge_scroll_desc().item_id().find(&self.item_id) {
                if scroll.auto_collect {
                    discovery.acquire_lore(ctx, scroll.item_id);
                    if scroll.secondary_knowledge_id != 0 {
                        discovery.acquire_secondary(ctx, scroll.secondary_knowledge_id);
                    }
                    self.quantity = 0; // this will consume all instances of this item but those stacks should only have 1 anyways
                }
            } else if let Some(food_desc) = ctx.db.food_desc().item_id().find(&self.item_id) {
                if food_desc.auto_consume {
                    food::consume(ctx, player_entity_id, &food_desc, self.quantity)
                        .expect("Auto-consume food failed to apply its effects or outputs");
                    self.quantity = 0;
                }
            }
        }
        true
    }

    pub fn get_item_list_id(&self, ctx: &ReducerContext) -> i32 {
        match &self.item_type {
            ItemType::Item => ctx.db.item_desc().id().find(&self.item_id).unwrap().item_list_id,
            ItemType::Cargo => 0,
        }
    }

    pub fn from(ctx: &ReducerContext, item_stack: &InputItemStack) -> Self {
        // Note: we assume input item stacks will always produce new item stacks (with full durability)
        Self::new(ctx, item_stack.item_id, item_stack.item_type, item_stack.quantity)
    }

    pub fn fix_durability(&self) -> ItemStack {
        let mut item_stack = self.clone();
        item_stack.fix_durability_self();
        item_stack
    }

    pub fn fix_durability_self(&mut self) {
        if self.durability == Some(0) {
            self.durability = None;
        }
    }

    // this will be useful if we refactor inventory operations
    pub fn process(&self, ctx: &ReducerContext, discovery: &mut Option<&mut Discovery>) -> Vec<ItemStack> {
        let mut player_id: Option<u64> = None;

        // Discover
        if discovery.is_some() {
            let d = discovery.as_mut().unwrap();
            d.acquire_item_stack(ctx, &self);
            player_id = Some(d.player_entity_id);
        }

        // Extract from Item List
        let mut new_item_stacks = ItemListDesc::extract_item_stacks_from_item(ctx, *self, player_id);

        // Discover and auto-collect definitive items
        if discovery.is_some() {
            let d = discovery.as_mut().unwrap();

            for item_stack in &mut new_item_stacks {
                if item_stack.item_type == ItemType::Item {
                    item_stack.auto_collect(ctx, d, d.player_entity_id);
                }

                d.acquire_item_stack(ctx, item_stack);
            }
            d.acquire_item_stack(ctx, &self);
        }

        // Fix Durability
        for item_stack in &mut new_item_stacks {
            item_stack.fix_durability_self();
        }

        new_item_stacks
    }

    pub fn process_all(ctx: &ReducerContext, item_stacks: Vec<ItemStack>, discovery: &mut Option<&mut Discovery>) -> Vec<ItemStack> {
        let mut new_item_stacks: Vec<ItemStack> = Vec::new();

        for item_stack in &item_stacks {
            let mut converted_item_stacks = item_stack.process(ctx, discovery);
            new_item_stacks.append(&mut converted_item_stacks);
        }
        new_item_stacks
    }
}

#[test]
pub fn test() {
    let mut stack_1 = ItemStack {
        item_id: 1,
        item_type: ItemType::Item,
        quantity: 1,
        durability: None,
    };
    let stack_2 = ItemStack {
        item_id: 2,
        item_type: ItemType::Item,
        quantity: 2,
        durability: None,
    };
    let stack_3 = ItemStack {
        item_id: 1,
        item_type: ItemType::Item,
        quantity: 3,
        durability: None,
    };

    assert_eq!(stack_1.merge(stack_2), false, "cannot merge, item_id are different");
    assert_eq!(stack_1.merge(stack_3), true, "should merge successfully, item_id are the same");
    assert_eq!(stack_1.quantity, 4, "stack should now have 4 items");

    let stack_4 = stack_1;
    assert_eq!(stack_4.quantity, 4, "copied stack should now have 4 items as well");

    stack_1.quantity -= 1;
    assert_eq!(stack_1.quantity, 3, "stack 1 should now have 3 items");
    assert_eq!(stack_4.quantity, 4, "copied stack should still have 4 items");

    let stacks_vec = ItemStack::merge_multiple(&vec![stack_1, stack_2, stack_3, stack_4]);
    assert_eq!(stacks_vec[0].quantity, 10, "stacks_vec[0] should now have 3 + 0 + 3 + 4 = 10 items");
    assert_eq!(stacks_vec[0].item_id, 1, "stacks_vec[0] item_id should be 1");
    assert_eq!(stacks_vec[1].quantity, 2, "stacks_vec[1] should now have 0 + 2 + 0 + 0 = 2 items");
    assert_eq!(stacks_vec[1].item_id, 2, "stacks_vec[1] item_id should be 2");
    assert_eq!(stacks_vec.iter().count(), 2, "stacks_vec should only have 2 elements in it");
}
