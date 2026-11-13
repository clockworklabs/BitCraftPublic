use spacetimedb::ReducerContext;

use crate::{
    cargo_desc, item_desc,
    messages::game_util::{ItemStack, ItemType, Pocket},
};

impl Pocket {
    pub fn available_space(&self, ctx: &ReducerContext, is_cargo: bool) -> i32 {
        let item_stack = match &self.contents {
            Some(stack) => stack,
            None => return self.volume,
        };

        let item_volume = if is_cargo {
            let cargo_def = ctx.db.cargo_desc().id().find(item_stack.item_id).unwrap();
            cargo_def.volume
        } else {
            let item_def = ctx.db.item_desc().id().find(item_stack.item_id).unwrap();
            item_def.volume
        };
        self.volume - item_stack.quantity * item_volume
    }

    pub fn can_fit_quantity(&self, ctx: &ReducerContext, volume: i32, is_cargo: bool) -> i32 {
        if self.volume <= 0 || volume <= 0 {
            return i32::MAX;
        }
        self.available_space(ctx, is_cargo) / volume
    }

    pub fn remove_quantity(&mut self, quantity: i32) -> i32 {
        if self.contents.is_none() {
            return 0;
        }

        let item_stack = self.contents.unwrap();
        if item_stack.quantity > quantity {
            let mut copy = item_stack;
            copy.quantity -= quantity;
            self.contents = Some(copy);
            return quantity;
        }

        self.contents = None;
        item_stack.quantity
    }

    pub fn add_quantity(&mut self, quantity: i32) {
        if let Some(item_stack) = self.contents.as_mut() {
            item_stack.quantity += quantity;
        }
    }

    pub fn set(&mut self, item_id: i32, item_type: ItemType, quantity: i32, durability: Option<i32>) {
        let item_stack = ItemStack {
            item_id,
            item_type,
            quantity,
            durability,
        };
        self.contents = Some(item_stack);
    }

    pub fn empty(volume: i32) -> Self {
        Pocket {
            volume,
            contents: None,
            locked: false,
        }
    }
}
