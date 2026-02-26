use std::collections::HashSet;

use crate::game::discovery::Discovery;
use crate::game::game_state::{create_entity, game_state_filters};
use crate::game::handlers::server::on_durability_zero::OnDurabilityZeroTimer;
use crate::game::reducer_helpers::cargo_helpers::spawn_cargo;
use crate::messages::components::*;
use crate::messages::game_util::*;
use crate::messages::static_data::*;
use crate::{unwrap_or_continue, unwrap_or_err, SmallHexTile};
use spacetimedb::rand::Rng;
use spacetimedb::{log, ReducerContext, Table};

pub struct RemoveItemResult {
    pub success: bool,
    pub inventory_changed: bool,
}

impl InventoryState {
    pub fn update(self, ctx: &ReducerContext) {
        ctx.db.inventory_state().entity_id().update(self);
    }

    pub fn insert(self, ctx: &ReducerContext) {
        ctx.db.inventory_state().insert(self);
    }

    pub fn is_pocket_cargo(&self, pocket_index: usize) -> bool {
        pocket_index >= self.cargo_index as usize
    }

    pub fn next_free_pocket(&self, item_type: ItemType) -> Option<usize> {
        return match item_type {
            ItemType::Item => self.next_free_item_pocket(),
            ItemType::Cargo => self.next_free_cargo_pocket(),
        };
    }

    pub fn next_free_cargo_pocket(&self) -> Option<usize> {
        for i in self.cargo_index as usize..self.pockets.len() {
            if self.is_pocket_empty(i) {
                return Some(i);
            }
        }
        None
    }

    pub fn next_free_item_pocket(&self) -> Option<usize> {
        for i in 0..self.cargo_index as usize {
            if self.is_pocket_empty(i) {
                return Some(i);
            }
        }
        None
    }

    pub fn new_with_index(
        ctx: &ReducerContext,
        num_pockets: i32,
        item_pocket_volume: i32,
        cargo_pocket_volume: i32,
        inventory_index: i32,
        cargo_index: i32,
        owner_entity_id: u64,
        player_owner_entity_id: u64,
        item_stacks: Option<Vec<ItemStack>>,
    ) -> bool {
        let entity_id = create_entity(ctx);

        let inventory = Self::create_with_pockets(
            entity_id,
            num_pockets,
            item_pocket_volume,
            cargo_pocket_volume,
            inventory_index,
            cargo_index,
            owner_entity_id,
            player_owner_entity_id,
            item_stacks,
        );

        if ctx.db.inventory_state().try_insert(inventory).is_err() {
            log::error!("Failed to insert inventory");
            return false;
        }

        true
    }

    pub fn new(
        ctx: &ReducerContext,
        num_pockets: i32,
        item_pocket_volume: i32,
        cargo_pocket_volume: i32,
        cargo_index: i32,
        owner_entity_id: u64,
        player_owner_entity_id: u64,
        item_stacks: Option<Vec<ItemStack>>,
    ) -> bool {
        Self::new_with_index(
            ctx,
            num_pockets,
            item_pocket_volume,
            cargo_pocket_volume,
            0,
            cargo_index,
            owner_entity_id,
            player_owner_entity_id,
            item_stacks,
        )
    }

    pub fn create_with_pockets(
        entity_id: u64,
        num_pockets: i32,
        item_pocket_volume: i32,
        cargo_pocket_volume: i32,
        inventory_index: i32,
        cargo_index: i32,
        owner_entity_id: u64,
        player_owner_entity_id: u64,
        item_stacks: Option<Vec<ItemStack>>,
    ) -> InventoryState {
        let mut pockets = Vec::new();
        if let Some(item_stacks) = item_stacks {
            for pocket_index in 0..(num_pockets as usize) {
                let pocket_volume = if pocket_index < cargo_index as usize {
                    item_pocket_volume
                } else {
                    cargo_pocket_volume
                };
                let contents = if item_stacks.len() > pocket_index {
                    Some(item_stacks[pocket_index])
                } else {
                    None
                };

                pockets.push(Pocket {
                    volume: pocket_volume,
                    contents,
                    locked: false,
                });
            }
        } else {
            for pocket_index in 0..num_pockets {
                let pocket_volume = if pocket_index < cargo_index {
                    item_pocket_volume
                } else {
                    cargo_pocket_volume
                };
                pockets.push(Pocket {
                    volume: pocket_volume,
                    contents: None,
                    locked: false,
                });
            }
        }

        InventoryState {
            entity_id,
            pockets,
            inventory_index,
            cargo_index,
            owner_entity_id,
            player_owner_entity_id,
        }
    }

    pub fn add_pockets(&mut self, num_new_pockets: i32, pocket_volume: i32) {
        for _ in 0..num_new_pockets {
            self.pockets.push(Pocket {
                volume: pocket_volume,
                contents: None,
                locked: false,
            });
        }
    }

    pub fn add(&mut self, ctx: &ReducerContext, item_stack: ItemStack) -> bool {
        let mut item_stack_copy = item_stack.clone();
        item_stack_copy.fix_durability();
        let mut inventory_copy = self.clone();
        inventory_copy.add_partial(ctx, &mut item_stack_copy);
        if item_stack_copy.quantity == 0 {
            self.pockets = inventory_copy.pockets;
            return true;
        }
        false
    }

    pub fn double(&mut self) {
        let mut new_pockets = Vec::new();

        for i in 0..self.cargo_index as usize {
            let pocket = &self.pockets[i];
            new_pockets.push(pocket.clone());
        }
        for i in 0..self.cargo_index as usize {
            let pocket = &self.pockets[i];
            new_pockets.push(Pocket::empty(pocket.volume));
        }

        for i in self.cargo_index as usize..self.pockets.len() {
            let pocket = &self.pockets[i];
            new_pockets.push(pocket.clone());
        }
        for i in self.cargo_index as usize..self.pockets.len() {
            let pocket = &self.pockets[i];
            new_pockets.push(Pocket::empty(pocket.volume));
        }
        self.pockets = new_pockets;
        self.inventory_index *= 2;
        self.cargo_index *= 2;
    }

    pub fn add_multiple(&mut self, ctx: &ReducerContext, item_stacks: &Vec<ItemStack>) -> bool {
        let mut inventory_copy = self.clone();
        for &stack in item_stacks {
            if !inventory_copy.add(ctx, stack) {
                return false;
            }
        }
        self.pockets = inventory_copy.pockets;
        true
    }

    pub fn add_multiple_partial(&mut self, ctx: &ReducerContext, item_stacks: &mut Vec<ItemStack>) {
        for stack in item_stacks {
            self.add_partial(ctx, stack);
        }
    }

    pub fn add_partial(&mut self, ctx: &ReducerContext, item_stack: &mut ItemStack) -> bool {
        if item_stack.quantity <= 0 {
            return false;
        }

        item_stack.fix_durability_self();

        let item_type = item_stack.item_type;
        let is_cargo = item_type == ItemType::Cargo;

        let volume: i32;

        let item_id = item_stack.item_id;
        if is_cargo {
            volume = match ctx.db.cargo_desc().id().find(&item_id) {
                Some(c) => c.volume,
                None => return false, // invalid cargo id, probably 0.
            };
        } else {
            volume = match ctx.db.item_desc().id().find(&item_id) {
                Some(i) => i.volume,
                None => return false, // invalid item id, probably 0.
            }
        }

        let mut has_filled_any = false;

        // Fill partial pockets first
        for p in &mut self.pockets {
            let content = match &p.contents {
                Some(c) => c,
                None => continue,
            };
            if content.item_id == item_id
                && content.item_type == item_type
                && content.durability.is_none()
                && item_stack.durability.is_none()
            {
                let inserted_count = std::cmp::min(p.can_fit_quantity(ctx, volume, is_cargo), item_stack.quantity);
                p.add_quantity(inserted_count);
                item_stack.quantity -= inserted_count;
                has_filled_any = true;
                if item_stack.quantity == 0 {
                    return true;
                }
            }
        }

        if item_stack.quantity > 0 {
            // Fill empty pockets next
            let num_pockets = self.pockets.len();
            for i in 0..num_pockets {
                if self.is_pocket_cargo(i) == is_cargo {
                    let p = &self.pockets[i];
                    match p.contents {
                        Some(_) => continue,
                        None => {
                            let inserted_count = std::cmp::min(p.can_fit_quantity(ctx, volume, is_cargo), item_stack.quantity);
                            self.pockets[i].set(item_id, item_type, inserted_count, item_stack.durability);
                            item_stack.quantity -= inserted_count;
                            has_filled_any = true;
                            if item_stack.quantity == 0 {
                                return true;
                            }
                        }
                    };
                }
            }
        }

        has_filled_any
    }

    pub fn add_partial_at(&mut self, ctx: &ReducerContext, item_stack: &mut ItemStack, pocket_index: usize) -> bool {
        if item_stack.quantity <= 0 {
            return false;
        }

        item_stack.fix_durability_self();

        let item_type = item_stack.item_type;
        let is_cargo = item_type == ItemType::Cargo;

        let volume: i32;

        let item_id = item_stack.item_id;
        if is_cargo {
            volume = match ctx.db.cargo_desc().id().find(&item_id) {
                Some(c) => c.volume,
                None => return false, // invalid cargo id, probably 0.
            };
        } else {
            volume = match ctx.db.item_desc().id().find(&item_id) {
                Some(i) => i.volume,
                None => return false, // invalid item id, probably 0.
            }
        }

        if self.is_pocket_cargo(pocket_index) != is_cargo {
            return false;
        }

        let mut has_filled_any = false;

        let p = self.pockets.get_mut(pocket_index).unwrap();

        if let Some(content) = p.contents {
            if content.item_id == item_id
                && content.item_type == item_type
                && content.durability.is_none()
                && item_stack.durability.is_none()
            {
                let inserted_count = std::cmp::min(p.can_fit_quantity(ctx, volume, is_cargo), item_stack.quantity);
                p.add_quantity(inserted_count);
                item_stack.quantity -= inserted_count;
                has_filled_any = true;
            }
        } else {
            let inserted_count = std::cmp::min(p.can_fit_quantity(ctx, volume, is_cargo), item_stack.quantity);
            self.pockets[pocket_index].set(item_id, item_type, inserted_count, item_stack.durability);
            item_stack.quantity -= inserted_count;
            has_filled_any = true;
        }

        has_filled_any
    }

    pub fn remove_partial(&mut self, item_stack: &mut ItemStack) -> bool {
        if item_stack.quantity <= 0 {
            return false;
        }

        let mut has_removed_any = false;

        for pocket in &mut self.pockets {
            if let Some(contents) = &mut pocket.contents {
                if contents.item_type == item_stack.item_type && contents.item_id == item_stack.item_id {
                    let quantity_to_remove = contents.quantity.min(item_stack.quantity);

                    contents.quantity -= quantity_to_remove;
                    item_stack.quantity -= quantity_to_remove;
                    has_removed_any = true;

                    if contents.quantity <= 0 {
                        pocket.contents = None;
                    }

                    //Nothing left to remove, exit early
                    if item_stack.quantity == 0 {
                        return true;
                    }
                }
            }
        }

        has_removed_any
    }

    pub fn is_pocket_empty(&self, pocket_index: usize) -> bool {
        match self.get_pocket_contents(pocket_index) {
            Some(_) => false,
            None => true,
        }
    }

    pub fn get_pocket_contents(&self, pocket_index: usize) -> Option<ItemStack> {
        match self.pockets.get(pocket_index) {
            Some(p) => p.contents,
            None => None,
        }
    }

    pub fn has(&self, item_stacks: &Vec<ItemStack>) -> bool {
        if item_stacks.is_empty() {
            return true;
        }

        let merged_stacks = ItemStack::merge_multiple(item_stacks);

        for stack in merged_stacks {
            let mut required = stack.quantity;

            for p in self.pockets.iter() {
                required -= match &p.contents {
                    Some(c) => {
                        // Need to check both item ID and type to ensure we're matching the correct item/cargo
                        // If durability is ever re-added as a game mechanic, should also be considered here
                        if c.item_id == stack.item_id && c.item_type == stack.item_type {
                            c.quantity
                        } else {
                            0
                        }
                    }
                    None => 0,
                };
            }

            if required > 0 {
                return false;
            }
        }

        true
    }

    pub fn fits(&self, ctx: &ReducerContext, item_stack: ItemStack) -> bool {
        self.fits_all(ctx, &vec![item_stack])
    }

    pub fn fits_all(&self, ctx: &ReducerContext, item_stacks: &Vec<ItemStack>) -> bool {
        let mut inventory_copy = self.clone();
        for &stack in item_stacks.iter() {
            if !inventory_copy.add(ctx, stack) {
                return false;
            }
        }
        true
    }

    pub fn fits_all_after_remove(&self, ctx: &ReducerContext, to_add: &Vec<ItemStack>, to_remove: &Vec<ItemStack>) -> bool {
        let mut inventory_copy = self.clone();
        inventory_copy.remove(to_remove);
        inventory_copy.fits_all(ctx, to_add)
    }

    pub fn is_empty(&self) -> bool {
        for p in self.pockets.iter() {
            match &p.contents {
                Some(c) => {
                    if c.quantity > 0 {
                        return false;
                    }
                }
                None => (),
            };
        }
        true
    }

    pub fn remove_input_stacks(&mut self, ctx: &ReducerContext, input_item_stacks: &Vec<InputItemStack>) -> RemoveItemResult {
        let item_stacks: Vec<ItemStack> = input_item_stacks
            .iter()
            .map(|iis| ItemStack::new(ctx, iis.item_id, iis.item_type, iis.quantity))
            .collect();

        let mut result = RemoveItemResult {
            success: false,
            inventory_changed: false,
        };

        // We still need to validate if the player has all the potential consumed items in inventory, even if some won't be consumed
        if !self.has(&item_stacks) {
            return result;
        }

        // trim item stacks that rolled a consumption result
        let consumed_item_stacks: Vec<ItemStack> = input_item_stacks
            .iter()
            .map(|iis| {
                let rnd = ctx.rng().gen_range(0.0..=1.0);
                if rnd <= iis.consumption_chance {
                    Some(ItemStack::new(ctx, iis.item_id, iis.item_type, iis.quantity))
                } else {
                    None
                }
            })
            .filter(|iis| iis.is_some())
            .map(|iis| iis.unwrap())
            .collect();

        result.success = self.remove(&consumed_item_stacks);
        result.inventory_changed = result.success && consumed_item_stacks.len() > 0;
        result
    }

    pub fn remove(&mut self, item_stacks: &Vec<ItemStack>) -> bool {
        let mut pockets_copy = self.pockets.clone();

        for &stack in item_stacks.iter() {
            if stack.quantity == 0 {
                continue;
            }
            if stack.quantity < 0 {
                return false;
            }
            let item_id = stack.item_id;
            let item_type = stack.item_type;
            let mut quantity = stack.quantity;

            while quantity > 0 {
                let mut smallest_pocket: Option<&mut Pocket> = None;

                // Compare current pocket with the smallest we found
                for p in &mut pockets_copy {
                    if let Some(content) = p.contents.as_ref() {
                        if content.item_id == item_id && content.item_type == item_type {
                            if let Some(sp) = &smallest_pocket {
                                if sp.contents.as_ref().unwrap().quantity <= content.quantity {
                                    // Our current smallest pocket is smaller than this new pocket's quantity, skip
                                    continue;
                                }
                            }
                            // No current smallest pocket, or its quantity is larger than this new pocket's quantity.
                            smallest_pocket = Some(p);
                        }
                    }
                }

                // Reduce quantity from our smallest pocket found
                match smallest_pocket {
                    None => {
                        return false;
                    }
                    Some(p) => {
                        let mut pocket_quantity = p.contents.as_ref().unwrap().quantity;
                        if p.contents.as_ref().unwrap().quantity >= quantity {
                            pocket_quantity -= quantity;
                            quantity = 0;
                            if pocket_quantity == 0 {
                                p.contents = None;
                            } else {
                                if let Some(c) = p.contents.as_mut() {
                                    c.quantity = pocket_quantity;
                                }
                            }
                        } else {
                            quantity -= pocket_quantity;
                            p.contents = None;
                        }
                    }
                }
            }
        }
        self.pockets = pockets_copy;
        true
    }

    pub fn remove_at(&mut self, pocket_index: usize) -> Option<ItemStack> {
        let pocket = self.pockets.get(pocket_index);
        if let Some(p) = pocket {
            if let Some(stack) = p.contents.as_ref() {
                let return_stack = stack.clone();
                self.set_at(pocket_index, None);
                return Some(return_stack);
            }
        };
        None
    }

    pub fn get_at(&self, pocket_index: usize) -> Option<&ItemStack> {
        let pocket = self.pockets.get(pocket_index);
        if let Some(p) = pocket {
            if let Some(stack) = p.contents.as_ref() {
                return Some(stack);
            }
        };
        None
    }

    pub fn remove_quantity_at(&mut self, pocket_index: usize, quantity: i32) -> Option<ItemStack> {
        if quantity < 0 {
            return None;
        }
        let pocket = self.pockets.get(pocket_index);
        if let Some(p) = pocket {
            if let Some(stack) = p.contents.as_ref() {
                let new_stack = stack.clone_with_quantity(stack.quantity - quantity);
                if new_stack.quantity < 0 {
                    return None;
                }
                let return_stack = Some(stack.clone_with_quantity(quantity));
                self.set_at(pocket_index, if new_stack.quantity == 0 { None } else { Some(new_stack) });
                return return_stack;
            }
        };
        None
    }

    pub fn add_at(&mut self, ctx: &ReducerContext, pocket_index: usize, contents: ItemStack) -> bool {
        let quantity = contents.quantity;

        if quantity <= 0 {
            return true;
        }

        let item_id = contents.item_id;
        let is_cargo = self.is_pocket_cargo(pocket_index);

        if is_cargo != (contents.item_type == ItemType::Cargo) {
            return false; // trying to add item to cargo slot or vice versa
        }

        let volume = if is_cargo {
            match ctx.db.cargo_desc().id().find(&item_id) {
                Some(c) => c.volume,
                None => return false, // invalid cargo id, probably 0.
            }
        } else {
            match ctx.db.item_desc().id().find(&item_id) {
                Some(i) => i.volume,
                None => return false, // invalid item id, probably 0.
            }
        };

        let pocket = &self.pockets[pocket_index];
        let inserted_quantity = std::cmp::min(pocket.can_fit_quantity(ctx, volume, is_cargo), quantity);

        let mut item_stack = contents.clone_with_quantity(inserted_quantity);
        item_stack.fix_durability();

        if pocket.contents.is_some() {
            let current = pocket.contents.unwrap();
            if current.item_id != item_id {
                return false;
            }
            item_stack.quantity += current.quantity;
        }

        let mut inventory_copy = self.clone();

        inventory_copy.set_at(pocket_index, Some(item_stack));

        if quantity > inserted_quantity {
            let remaining_stack = contents.clone_with_quantity(quantity - inserted_quantity);
            if !inventory_copy.add(ctx, remaining_stack) {
                return false;
            }
        }

        self.pockets = inventory_copy.pockets;

        return true;
    }

    pub fn set_at(&mut self, pocket_index: usize, contents: Option<ItemStack>) {
        let pocket = self.pockets.get(pocket_index);

        // set an empty pocket instead of a pocket with a quantity of 0
        let pocket_contents = match contents {
            Some(c) => {
                if c.quantity <= 0 {
                    None
                } else {
                    Some(c.fix_durability())
                }
            }
            None => None,
        };

        if let Some(p) = pocket {
            self.pockets[pocket_index] = Pocket {
                contents: pocket_contents,
                volume: p.volume,
                locked: false,
            };
        }
    }

    pub fn pocket_with_lowest_count(&self, item_id: i32) -> i32 {
        let mut lowest_index = -1;
        let mut lowest_quantity = i32::MAX;
        for i in 0..self.pockets.len() {
            if let Some(contents) = self.pockets[i].contents {
                if contents.item_id == item_id && contents.quantity > 0 && contents.quantity <= lowest_quantity {
                    lowest_quantity = contents.quantity;
                    lowest_index = i as i32;
                }
            }
        }
        lowest_index
    }

    pub fn lock_pocket(&mut self, pocket_index: usize) {
        self.pockets[pocket_index].locked = true;
    }

    pub fn unlock_pocket(&mut self, pocket_index: usize) {
        self.pockets[pocket_index].locked = false;
    }

    pub fn unlock_all_pockets(&mut self) {
        for i in 0..self.pockets.len() {
            self.unlock_pocket(i);
        }
    }

    pub fn get_player_inventory(ctx: &ReducerContext, player_entity_id: u64) -> Option<InventoryState> {
        Self::get_by_owner_with_index(ctx, player_entity_id, 0)
    }

    pub fn get_player_toolbelt(ctx: &ReducerContext, player_entity_id: u64) -> Option<InventoryState> {
        Self::get_by_owner_with_index(ctx, player_entity_id, 1)
    }

    pub fn get_player_wallet(ctx: &ReducerContext, player_entity_id: u64) -> Option<InventoryState> {
        Self::get_by_owner_with_index(ctx, player_entity_id, 2)
    }

    pub fn wallet_pocket_target(ctx: &ReducerContext, item_stack: &ItemStack) -> Option<usize> {
        if item_stack.item_type == ItemType::Cargo {
            return None;
        }
        if item_stack.item_id == ItemStack::hex_coins(0).item_id {
            return Some(0);
        }
        if item_stack.item_id == ItemStack::empire_currency(ctx, 0).item_id {
            return Some(1);
        }
        return None;
    }

    pub fn reduce_tool_durability(ctx: &ReducerContext, player_entity_id: u64, tool_type: i32, durability_lost: i32) {
        let mut toolbelt = InventoryState::get_player_toolbelt(ctx, player_entity_id).unwrap();
        let toolbelt_pocket_index = ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool_type) as usize;
        if let Some(pocket) = toolbelt.pockets.get_mut(toolbelt_pocket_index) {
            if let Some(equipped_tool) = pocket.contents.as_mut() {
                let equipped_item_id = equipped_tool.item_id;
                if let Some(durability) = equipped_tool.durability.as_mut() {
                    *durability -= durability_lost;
                    if *durability <= 0 {
                        let mut still_equipped = false;
                        let mut added_to_inventory = false;
                        // Item either gets destroyed or morph into another and possibly gets put in inventory
                        let convert_into = ctx
                            .db
                            .item_desc()
                            .id()
                            .find(equipped_tool.item_id)
                            .unwrap()
                            .convert_to_on_durability_zero;
                        if convert_into == 0 {
                            toolbelt.pockets[toolbelt_pocket_index].contents = None;
                            PlayerState::on_removed_from_toolbelt(ctx, player_entity_id, equipped_item_id);
                        } else {
                            let created_item = ItemStack::new(ctx, convert_into, ItemType::Item, 1);

                            let mut discovery = Discovery::new(player_entity_id);
                            discovery.acquire_item(ctx, convert_into);
                            discovery.commit(ctx);

                            if ctx
                                .db
                                .tool_desc()
                                .item_id()
                                .filter(convert_into)
                                .filter(|td| td.tool_type == tool_type)
                                .next()
                                .is_some()
                            {
                                // Converts into a similar type of tool that can still be equipped
                                toolbelt.pockets[toolbelt_pocket_index].contents = Some(created_item);
                                still_equipped = true;
                                PlayerState::on_updated_toolbelt(ctx, player_entity_id, equipped_item_id, convert_into);
                            } else {
                                // Converts into something that cannot be equipped on this slot.
                                toolbelt.pockets[toolbelt_pocket_index].contents = None;
                                let mut inventory = InventoryState::get_player_inventory(ctx, player_entity_id).unwrap();
                                added_to_inventory = inventory.fits(ctx, created_item);
                                inventory.add_multiple_with_overflow(ctx, &vec![created_item]); // warning will be set by client
                                ctx.db.inventory_state().entity_id().update(inventory);
                                PlayerState::on_removed_from_toolbelt(ctx, player_entity_id, equipped_item_id);
                            }
                        }
                        OnDurabilityZeroTimer::schedule(
                            ctx,
                            player_entity_id,
                            equipped_item_id,
                            convert_into,
                            still_equipped,
                            added_to_inventory,
                        );
                    }
                    ctx.db.inventory_state().entity_id().update(toolbelt);
                }
            }
        }
    }

    pub fn add_to_player_wallet_and_commit(
        ctx: &ReducerContext,
        player_entity_id: u64,
        coin_amount: i32,
        empire_currency_amount: i32,
    ) -> bool {
        if let Some(mut wallet) = Self::get_player_wallet(ctx, player_entity_id) {
            if coin_amount > 0 {
                let coins = ItemStack::hex_coins(coin_amount);
                if !wallet.add_at(ctx, 0, coins) {
                    return false;
                }
            }
            if empire_currency_amount > 0 {
                let empire_currency = ItemStack::empire_currency(ctx, empire_currency_amount);
                if !wallet.add_at(ctx, 1, empire_currency) {
                    return false;
                }
            }
            ctx.db.inventory_state().entity_id().update(wallet);
            return true;
        }
        false
    }

    pub fn get_by_owner(ctx: &ReducerContext, owner_entity_id: u64) -> Option<InventoryState> {
        Self::get_by_owner_with_index(ctx, owner_entity_id, 0)
    }

    pub fn get_by_owner_with_index(ctx: &ReducerContext, owner_entity_id: u64, inventory_index: i32) -> Option<InventoryState> {
        ctx.db
            .inventory_state()
            .owner_entity_id()
            .filter(owner_entity_id)
            .find(|inv: &InventoryState| inv.inventory_index == inventory_index)
    }

    pub fn get_player_cargo_id(ctx: &ReducerContext, player_entity_id: u64) -> i32 {
        if let Some(inventory) = Self::get_by_owner(ctx, player_entity_id) {
            if let Some(cargo) = inventory.get_at(inventory.cargo_index as usize) {
                return cargo.item_id;
            }
        }
        0
    }

    pub fn update_remove_player_cargo(ctx: &ReducerContext, player_entity_id: u64) -> bool {
        let mut inventory = Self::get_by_owner(ctx, player_entity_id).unwrap();
        inventory.remove_at(inventory.cargo_index as usize);
        ctx.db.inventory_state().entity_id().update(inventory);
        true
    }

    pub fn add_and_discover(
        ctx: &ReducerContext,
        player_entity_id: u64,
        discovery: &mut Discovery,
        item_stack: ItemStack,
        dry_run: bool,
    ) -> bool {
        if item_stack.quantity <= 0 {
            return false;
        }

        let item_stack = item_stack.fix_durability();

        //hex coin use wallet first
        let wallet_pocket = InventoryState::wallet_pocket_target(ctx, &item_stack);

        let mut inventory = if wallet_pocket.is_some() {
            InventoryState::get_player_wallet(ctx, player_entity_id).unwrap()
        } else {
            InventoryState::get_by_owner(ctx, player_entity_id).unwrap()
        };

        if item_stack.item_type == ItemType::Item {
            //handle item list items
            if let Some(item_desc) = ctx.db.item_desc().id().find(&item_stack.item_id) {
                if item_desc.item_list_id != 0 {
                    let converted_item_stacks = ItemListDesc::extract_item_stacks(ctx, item_desc.item_list_id);

                    // Attempt adding the extra converted items wherever you can in the inventory
                    if !dry_run {
                        for i in 0..converted_item_stacks.len() {
                            let mut item_stack = converted_item_stacks[i].clone();
                            discovery.acquire_item_stack(ctx, &item_stack);
                            item_stack.auto_collect(ctx, discovery, player_entity_id);
                            if item_stack.quantity > 0 {
                                for _ in 0..item_stack.quantity {
                                    if let Some(wallet_pocket_index) = wallet_pocket.as_ref() {
                                        inventory.add_at(ctx, *wallet_pocket_index, item_stack);
                                    } else {
                                        inventory.add_multiple_with_overflow(ctx, &vec![item_stack]);
                                    }
                                }
                            }
                        }
                    }

                    return if !dry_run {
                        ctx.db.inventory_state().entity_id().update(inventory);
                        true
                    } else {
                        false
                    };
                }
            }
        }

        if let Some(wallet_pocket_index) = wallet_pocket {
            let ret = inventory.add_at(ctx, wallet_pocket_index, item_stack);
            if !dry_run {
                discovery.acquire_item_stack(ctx, &item_stack);
                ctx.db.inventory_state().entity_id().update(inventory);
            }
            return ret;
        }

        if inventory.add(ctx, item_stack) {
            if !dry_run {
                discovery.acquire_item_stack(ctx, &item_stack);
                ctx.db.inventory_state().entity_id().update(inventory);
            }
            return true;
        }
        false
    }

    pub fn add_self(&mut self, ctx: &ReducerContext, item_stack: ItemStack) -> bool {
        self.add(ctx, item_stack)
    }

    pub fn add_and_discover_self(&mut self, ctx: &ReducerContext, discovery: &mut Discovery, item_stack: ItemStack) -> bool {
        if self.add(ctx, item_stack) {
            discovery.acquire_item_stack(ctx, &item_stack);
            true
        } else {
            false
        }
    }

    pub fn add_multiple_with_overflow(&mut self, ctx: &ReducerContext, item_stacks: &Vec<ItemStack>) {
        let mut inventory_copy = self.clone();
        let mut overflow_items: Vec<ItemStack> = Vec::new(); // Items that cannot fit into the player's inventory
        for stack in item_stacks {
            let mut stack = stack.clone();
            stack.fix_durability_self();
            inventory_copy.add_partial(ctx, &mut stack);
            if stack.quantity > 0 {
                overflow_items.push(stack);
            }
        }
        self.pockets = inventory_copy.pockets;

        if !overflow_items.is_empty() {
            let player_coordinates = game_state_filters::coordinates_any(ctx, self.owner_entity_id);
            DroppedInventoryState::update_from_items(ctx, self.owner_entity_id, player_coordinates.into(), overflow_items, None);
        }
    }

    pub fn remove_stacks_from_player_inventory(
        ctx: &ReducerContext,
        player_entity_id: u64,
        item_stacks: &Vec<ItemStack>,
        full_durability_only: bool,
    ) -> bool {
        let mut success_count = 0;

        let mut wallet_updated = false;
        let mut inventory_updated = false;
        if let Some(mut inventory) = InventoryState::get_player_inventory(ctx, player_entity_id) {
            let mut used_items = None;

            if full_durability_only {
                // We exclude any used durability items from the inventory for this operation
                used_items = Some(inventory.exclude_used_items(ctx));
            }

            if let Some(mut wallet) = InventoryState::get_player_wallet(ctx, player_entity_id) {
                for &stack_to_remove in item_stacks.iter() {
                    // fail on negative quantities
                    if stack_to_remove.quantity < 0 {
                        log::warn!("Attempted to remove item stack with negative quantity from player inventory");
                        return false;
                    }

                    if stack_to_remove.quantity == 0 {
                        success_count += 1;
                        continue;
                    }

                    let wallet_pocket = InventoryState::wallet_pocket_target(ctx, &stack_to_remove);

                    //wallet item
                    if let Some(wallet_pocket_index) = wallet_pocket {
                        let mut quantity_to_remove = stack_to_remove.quantity;

                        //wallet has coins
                        if let Some(current_wallet_pocket) = wallet.pockets.get(wallet_pocket_index).as_mut() {
                            if let Some(wallet_coin_stack) = current_wallet_pocket.contents {
                                //wallet quantity less than amount to remove, attempt to remove from wallet and inventory
                                if wallet_coin_stack.quantity < quantity_to_remove {
                                    let pocket_quantity = wallet_coin_stack.quantity;

                                    //sub amount from wallet
                                    quantity_to_remove -= pocket_quantity;

                                    let mut stack_sub_wallet = stack_to_remove.clone();
                                    stack_sub_wallet.quantity = quantity_to_remove;

                                    //remove remaining amount form inventory
                                    if inventory.remove(&vec![stack_sub_wallet]) {
                                        //all coins removed from wallet
                                        wallet.remove(&vec![wallet_coin_stack]);

                                        inventory_updated = true;
                                        wallet_updated = true;

                                        success_count += 1;
                                        continue;
                                    } else {
                                        break;
                                    };
                                }
                                //amount in wallet greater, remove all from wallet
                                else if wallet.remove(&vec![stack_to_remove]) {
                                    wallet_updated = true;

                                    success_count += 1;
                                    continue;
                                }
                            }
                        }
                    }

                    //no coins in wallet, remove normally
                    if inventory.remove(&vec![stack_to_remove]) {
                        inventory_updated = true;

                        success_count += 1;
                        continue;
                    }
                    break;
                }

                if success_count == item_stacks.len() {
                    if inventory_updated {
                        // Bring back low durability items in the inventory if it changed
                        if let Some(used_items) = used_items {
                            inventory.merge_used_items(&used_items);
                        }
                        ctx.db.inventory_state().entity_id().update(inventory);
                    }

                    if wallet_updated {
                        ctx.db.inventory_state().entity_id().update(wallet);
                    }
                }
            }
        }

        return success_count == item_stacks.len();
    }

    pub fn add_partial_and_discover(
        ctx: &ReducerContext,
        player_entity_id: u64,
        discovery: &mut Discovery,
        item_stack: &mut ItemStack,
    ) -> bool {
        if InventoryState::wallet_pocket_target(ctx, item_stack).is_some() {
            let coin_amount = if item_stack.item_id == ItemStack::hex_coins(0).item_id {
                item_stack.quantity
            } else {
                0
            };
            discovery.acquire_item(ctx, item_stack.item_id);
            let empire_currency_amount = if coin_amount == 0 { item_stack.quantity } else { 0 };
            InventoryState::add_to_player_wallet_and_commit(ctx, player_entity_id, coin_amount, empire_currency_amount);
            return true;
        }

        let mut inventory = InventoryState::get_player_inventory(ctx, player_entity_id).unwrap();
        if InventoryState::add_partial_to_inventory_and_discover(ctx, player_entity_id, discovery, &mut inventory, item_stack, true) {
            ctx.db.inventory_state().entity_id().update(inventory);
            return true;
        }
        return false;
    }

    pub fn add_partial_to_inventory_and_discover(
        ctx: &ReducerContext,
        player_entity_id: u64,
        discovery: &mut Discovery,
        inventory: &mut InventoryState,
        item_stack: &mut ItemStack,
        do_discover: bool,
    ) -> bool {
        if item_stack.item_type == ItemType::Item {
            //handle item list items
            if let Some(item_desc) = ctx.db.item_desc().id().find(&item_stack.item_id) {
                if item_desc.item_list_id != 0 {
                    let converted_item_stacks = ItemListDesc::extract_item_stacks(ctx, item_desc.item_list_id);

                    for _ in 0..item_stack.quantity {
                        for i in 0..converted_item_stacks.len() {
                            let mut item_stack = converted_item_stacks[i].clone();
                            item_stack.auto_collect(ctx, discovery, player_entity_id);
                            inventory.add_multiple_with_overflow(ctx, &vec![item_stack]);
                            if do_discover {
                                discovery.acquire_item_stack(ctx, &item_stack);
                            }
                        }
                    }

                    // deplete the item list.
                    item_stack.quantity = 0;
                    return true;
                }
            }
        }

        inventory.add_partial(ctx, item_stack);
        if do_discover {
            discovery.acquire_item_stack(ctx, &item_stack);
        }

        return true;
    }

    pub fn set_at_and_discover(
        ctx: &ReducerContext,
        pocket_index: usize,
        contents: Option<ItemStack>,
        discovery: &mut Discovery,
        inventory: &mut InventoryState,
        do_discover: bool,
    ) {
        let pocket = inventory.pockets.get(pocket_index);

        // set an empty pocket instead of a pocket with a quantity of 0
        let pocket_contents = match contents {
            Some(c) => {
                if c.quantity == 0 {
                    None
                } else {
                    contents
                }
            }
            None => None,
        };

        if let Some(p) = pocket {
            if let Some(item_stack) = pocket_contents {
                if item_stack.item_type == ItemType::Item {
                    //handle item list items
                    if let Some(item_desc) = ctx.db.item_desc().id().find(&item_stack.item_id) {
                        if item_desc.item_list_id != 0 {
                            let converted_item_stacks = ItemListDesc::extract_item_stacks(ctx, item_desc.item_list_id);

                            for _ in 0..item_stack.quantity {
                                // Attempt adding the extra converted items wherever you can in the inventory
                                for i in 0..converted_item_stacks.len() {
                                    let mut item_stack = converted_item_stacks[i].clone();
                                    item_stack.auto_collect(ctx, discovery, discovery.player_entity_id);
                                    inventory.add_multiple_with_overflow(ctx, &vec![item_stack]);

                                    //discover
                                    if do_discover {
                                        discovery.acquire_item_stack(ctx, &item_stack);
                                    }
                                }
                            }

                            //remove quantity
                            inventory.remove_quantity_at(pocket_index, item_stack.quantity);
                            return;
                        }
                    }
                }

                //discover
                if do_discover {
                    discovery.acquire_item_stack(ctx, &item_stack);
                }
            }

            inventory.pockets[pocket_index] = Pocket {
                contents: pocket_contents,
                volume: p.volume,
                locked: false,
            };
        }
    }

    pub fn coins(&self) -> i32 {
        self.pockets
            .iter()
            .filter_map(|p| match p.contents {
                Some(c) => {
                    if c.item_type == ItemType::Item && c.item_id == TradeOrderState::MARKET_MODE_CURRENCY_ID {
                        Some(c.quantity)
                    } else {
                        None
                    }
                }
                None => None,
            })
            .sum()
    }

    pub fn get_nearby_deployable_inventories<TDistanceFn>(
        ctx: &ReducerContext,
        player_entity_id: u64,
        get_distance_fn: TDistanceFn,
        max_distance: i32,
    ) -> Vec<InventoryState>
    where
        TDistanceFn: Fn(SmallHexTile) -> i32,
    {
        let mut inventories_with_distance: Vec<(InventoryState, i32)> = Vec::new();

        for deployable in ctx.db.deployable_state().owner_id().filter(player_entity_id) {
            if deployable.hidden {
                continue;
            }

            //Deployables that have been collected to the Vault will have a DeployableState but no MobileEntityState
            let Some(mobile_entity_state) = ctx.db.mobile_entity_state().entity_id().find(&deployable.entity_id) else {
                continue;
            };

            let distance = get_distance_fn(mobile_entity_state.coordinates());
            if distance > max_distance {
                continue;
            }

            let inventory = unwrap_or_continue!(
                InventoryState::get_by_owner(ctx, deployable.entity_id),
                "Deployable {} has no InventoryState",
                deployable.entity_id
            );

            inventories_with_distance.push((inventory, distance));
        }

        if inventories_with_distance.len() == 0 {
            return Vec::new();
        }

        inventories_with_distance.sort_by(|a, b| a.1.cmp(&b.1));
        return inventories_with_distance.into_iter().map(|x| x.0).collect();
    }

    pub fn withdraw_from_player_inventory_and_nearby_deployables<TDistanceFn>(
        ctx: &ReducerContext,
        player_entity_id: u64,
        item_stacks: &Vec<ItemStack>,
        get_distance_fn: TDistanceFn,
    ) -> Result<(), String>
    where
        TDistanceFn: Fn(SmallHexTile) -> i32,
    {
        //Attempt to withdraw everything from the player's inventory first
        if Self::remove_stacks_from_player_inventory(ctx, player_entity_id, item_stacks, false) {
            return Ok(());
        }

        //Find the player's nearby deployables and get their inventories
        let max_distance = ctx.db.parameters_desc().version().find(&0).unwrap().withdraw_from_deployables_range;
        let mut inventories = Self::get_nearby_deployable_inventories(ctx, player_entity_id, get_distance_fn, max_distance);

        if inventories.len() == 0 {
            return Err("You don't have the required items.".into());
        }

        let player_wallet = unwrap_or_err!(InventoryState::get_player_wallet(ctx, player_entity_id), "Player has no wallet");
        let player_inventory = unwrap_or_err!(
            InventoryState::get_player_inventory(ctx, player_entity_id),
            "Player has no inventory"
        );
        inventories.insert(0, player_wallet);
        inventories.insert(1, player_inventory);
        let mut changed_inventory_indices = HashSet::new();

        //Partially remove ItemStacks from the player's wallet, inventory, nearby deployables and keep track of changing inventories
        for item_stack in &mut item_stacks.clone() {
            for (inventory_index, inventory) in inventories.iter_mut().enumerate() {
                if inventory.remove_partial(item_stack) {
                    changed_inventory_indices.insert(inventory_index);
                }
            }

            if item_stack.quantity > 0 {
                return Err("You don't have the required items.".into());
            }
        }

        //Update inventories that have changed
        for (index, inventory) in inventories.into_iter().enumerate() {
            if changed_inventory_indices.contains(&index) {
                ctx.db.inventory_state().entity_id().update(inventory);
            }
        }

        Ok(())
    }

    pub fn deposit_to_player_inventory_and_nearby_deployables_and_get_overflow_stack<TDistanceFn>(
        ctx: &ReducerContext,
        player_entity_id: u64,
        item_stacks: &Vec<ItemStack>,
        get_distance_fn: TDistanceFn,
        force_fill_player_inventory: bool,
    ) -> Result<Vec<ItemStack>, String>
    where
        TDistanceFn: Fn(SmallHexTile) -> i32,
    {
        let mut discovery = Discovery::new(player_entity_id);
        let mut output = Vec::new();

        //Resolve ItemLists and handle discovery first
        for item_stack in item_stacks {
            let item_list_id = item_stack.get_item_list_id(ctx);
            if item_list_id == 0 {
                output.push(*item_stack);
                continue;
            }

            for _ in 0..item_stack.quantity {
                output.extend(ItemListDesc::extract_item_stacks(ctx, item_list_id));
            }

            discovery.acquire_item_stack(ctx, &item_stack);
        }
        discovery.acquire_item_stacks(ctx, &output);

        // Auto-collect scrolls
        for item_stack in &mut output {
            item_stack.auto_collect(ctx, &mut discovery, player_entity_id);
        }

        let player_settings = ctx.db.player_settings_state().entity_id().find(player_entity_id);
        let player_wallet = unwrap_or_err!(InventoryState::get_player_wallet(ctx, player_entity_id), "Player has no wallet");
        let player_inventory = unwrap_or_err!(
            InventoryState::get_player_inventory(ctx, player_entity_id),
            "Player has no inventory"
        );
        let max_distance = ctx.db.parameters_desc().version().find(&0).unwrap().withdraw_from_deployables_range;
        let mut inventories = Self::get_nearby_deployable_inventories(ctx, player_entity_id, get_distance_fn, max_distance);
        let mut changed_inventory_indices = HashSet::new();
        let mut overflow_items = Vec::new();

        if force_fill_player_inventory || player_settings.as_ref().map(|x| x.fill_player_inventory).unwrap_or(true) {
            inventories.insert(0, player_wallet); //Wallet first

            if player_settings.map(|x| x.fill_deployable_inventory_first).unwrap_or(false) {
                inventories.push(player_inventory); //Inventory last
            } else {
                inventories.insert(1, player_inventory); //Inventory second
            }
        }

        //Partially add ItemStacks from the player's wallet, inventory, nearby deployables and keep track of changing inventories and overflowing items
        for item_stack_index in (0..output.len()).rev() {
            let item_stack = &mut output[item_stack_index];

            for (inventory_index, inventory) in inventories.iter_mut().enumerate() {
                //Ensure only wallet items go into the wallet
                let wallet_pocket = InventoryState::wallet_pocket_target(ctx, item_stack);
                if let Some(wallet_pocket_ix) = wallet_pocket {
                    if inventory.add_at(ctx, wallet_pocket_ix, *item_stack) {
                        changed_inventory_indices.insert(inventory_index);
                        item_stack.quantity = 0;
                        break;
                    }
                } else if inventory_index > 0 {
                    // non-wallet items go in non-wallet inventories
                    if inventory.add_partial(ctx, item_stack) {
                        changed_inventory_indices.insert(inventory_index);
                        if item_stack.quantity <= 0 {
                            break;
                        }
                    }
                }
            }
            if item_stack.quantity > 0 {
                let item_stack = output.remove(item_stack_index);
                overflow_items.push(item_stack);
            }
        }

        discovery.commit(ctx);

        //Update inventories that have changed
        for (index, inventory) in inventories.into_iter().enumerate() {
            if changed_inventory_indices.contains(&index) {
                ctx.db.inventory_state().entity_id().update(inventory);
            }
        }
        Ok(overflow_items)
    }

    pub fn deposit_to_player_inventory_and_nearby_deployables<TDistanceFn, TCargoDropLocationsFn>(
        ctx: &ReducerContext,
        player_entity_id: u64,
        item_stacks: &Vec<ItemStack>,
        get_distance_fn: TDistanceFn,
        drop_overflow: bool,
        get_cargo_drop_location_fn: TCargoDropLocationsFn,
        force_fill_player_inventory: bool,
    ) -> Result<(), String>
    where
        TDistanceFn: Fn(SmallHexTile) -> i32,
        TCargoDropLocationsFn: FnOnce() -> Vec<SmallHexTile>,
    {
        let overflow_items = Self::deposit_to_player_inventory_and_nearby_deployables_and_get_overflow_stack(
            ctx,
            player_entity_id,
            item_stacks,
            get_distance_fn,
            force_fill_player_inventory,
        )?;

        //Create an item-pile for overflow items
        if overflow_items.len() > 0 {
            if !drop_overflow {
                return Err("~Unable to drop overflow items".into());
            }

            let player_location = unwrap_or_err!(
                ctx.db.mobile_entity_state().entity_id().find(&player_entity_id),
                "Unknown player location"
            );

            let cargo_drop_locations = get_cargo_drop_location_fn();

            for overflow_cargo in overflow_items.iter().filter(|of| of.item_type == ItemType::Cargo) {
                //Drop overflow cargo
                if cargo_drop_locations.len() == 1 {
                    spawn_cargo(
                        ctx,
                        player_entity_id,
                        cargo_drop_locations[0],
                        overflow_cargo.item_id,
                        overflow_cargo.quantity,
                    );
                    continue;
                }

                for _ in 0..overflow_cargo.quantity {
                    let drop_location = cargo_drop_locations[ctx.rng().gen_range(0..cargo_drop_locations.len()) as usize];
                    spawn_cargo(ctx, player_entity_id, drop_location, overflow_cargo.item_id, 1);
                }
            }

            let overflow_items: Vec<ItemStack> = overflow_items.into_iter().filter(|of| of.item_type == ItemType::Item).collect();
            DroppedInventoryState::update_from_items(ctx, player_entity_id, player_location.coordinates(), overflow_items, None);
        }
        Ok(())
    }

    pub fn get_all_content(&self) -> Vec<ItemStack> {
        self.pockets.iter().filter_map(|p| p.contents).collect()
    }

    pub fn exclude_used_items(&mut self, ctx: &ReducerContext) -> InventoryState {
        let mut copy = self.clone();

        for i in 0..self.pockets.len() {
            if let Some(content) = self.pockets[i].contents {
                if let Some(durability) = content.durability {
                    if durability < ctx.db.item_desc().id().find(content.item_id).unwrap().durability {
                        // Used durability item, we remove this from the original inventory; the used_inventory keeps its copy
                        self.pockets[i].contents = None;
                    } else {
                        // Full durability item, the used inventory won't retain its copy
                        copy.pockets[i].contents = None;
                    }
                } else {
                    // Not a durability item, the used inventory won't retain its copy
                    copy.pockets[i].contents = None;
                }
            }
        }
        copy
    }

    pub fn merge_used_items(&mut self, used_inventory: &InventoryState) {
        for i in 0..used_inventory.pockets.len() {
            if let Some(content) = used_inventory.pockets[i].contents {
                self.pockets[i].contents = Some(content);
            }
        }
    }

    pub fn withdraw_full_durability_from_player_inventory_and_nearby_deployables<TDistanceFn>(
        ctx: &ReducerContext,
        player_entity_id: u64,
        item_stacks: &Vec<ItemStack>,
        get_distance_fn: TDistanceFn,
    ) -> Result<(), String>
    where
        TDistanceFn: Fn(SmallHexTile) -> i32,
    {
        //Attempt to withdraw everything from the player's inventory first
        if Self::remove_stacks_from_player_inventory(ctx, player_entity_id, item_stacks, true) {
            return Ok(());
        }

        //Find the player's nearby deployables and get their inventories
        let max_distance = ctx.db.parameters_desc().version().find(&0).unwrap().withdraw_from_deployables_range;
        let mut inventories = Self::get_nearby_deployable_inventories(ctx, player_entity_id, get_distance_fn, max_distance);

        if inventories.len() == 0 {
            return Err("You don't have the required items.".into());
        }

        let player_wallet = unwrap_or_err!(InventoryState::get_player_wallet(ctx, player_entity_id), "Player has no wallet");
        let player_inventory = unwrap_or_err!(
            InventoryState::get_player_inventory(ctx, player_entity_id),
            "Player has no inventory"
        );
        inventories.insert(0, player_wallet);
        inventories.insert(1, player_inventory);

        // Remove all used durability items from those inventories
        let mut used_inventories = Vec::new();
        for i in 0..inventories.len() {
            let inv = inventories.get_mut(i).unwrap();
            used_inventories.push(inv.exclude_used_items(ctx));
        }

        let mut changed_inventory_indices = HashSet::new();

        //Partially remove ItemStacks from the player's wallet, inventory, nearby deployables and keep track of changing inventories
        for item_stack in &mut item_stacks.clone() {
            for (inventory_index, inventory) in inventories.iter_mut().enumerate() {
                if inventory.remove_partial(item_stack) {
                    changed_inventory_indices.insert(inventory_index);
                }
            }

            if item_stack.quantity > 0 {
                return Err("You don't have the required items.".into());
            }
        }

        //Update inventories that have changed
        for (index, mut inventory) in inventories.into_iter().enumerate() {
            if changed_inventory_indices.contains(&index) {
                // Restore  all used durability items from those inventories
                inventory.merge_used_items(&used_inventories[index]);
                ctx.db.inventory_state().entity_id().update(inventory);
            }
        }

        Ok(())
    }

    pub fn as_item_stacks(&self) -> Vec<ItemStack> {
        let inv = self
            .pockets
            .iter()
            .filter_map(|p| match p.contents {
                Some(c) => {
                    if c.quantity > 0 {
                        Some(c)
                    } else {
                        None
                    }
                }
                None => None,
            })
            .collect();
        inv
    }
}
