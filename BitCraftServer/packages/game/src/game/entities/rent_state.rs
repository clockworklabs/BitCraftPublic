use spacetimedb::ReducerContext;

use crate::{
    inventory_state,
    messages::{components::RentState, game_util::ItemStack},
    parameters_desc, unwrap_or_err,
};

use super::building_state::InventoryState;

impl RentState {
    pub fn is_renter(&self, player_entity_id: u64) -> bool {
        *self.white_list.first().unwrap_or(&0) == player_entity_id
    }

    pub fn is_tenant(&self, player_entity_id: u64) -> bool {
        self.white_list.contains(&player_entity_id)
    }

    pub fn pay_rent(&mut self, ctx: &ReducerContext, player_entity_id: u64, amount: u32) -> Result<(), String> {
        let max_amount = self.daily_rent * ctx.db.parameters_desc().version().find(0).unwrap().max_rental_deposit_days as u32;
        if self.paid_rent >= max_amount {
            return Err("Funds are already at maximum capacity".into());
        }
        let amount = amount.min(max_amount - self.paid_rent);

        //  self.paid_rent
        let coin_deposit = vec![ItemStack::hex_coins(amount as i32)];

        if !InventoryState::remove_stacks_from_player_inventory(ctx, player_entity_id, &coin_deposit, false) {
            return Err("You don't have enough funds.".into());
        }

        self.paid_rent += amount;

        Ok(())
    }

    pub fn pay_remaining_eviction_fee(
        &self,
        ctx: &ReducerContext,
        player_entity_id: u64,
        initial_amount: u32,
        remaining_amount: u32,
    ) -> Result<(), String> {
        let coin_deposit = vec![ItemStack::hex_coins(remaining_amount as i32)];

        if !InventoryState::remove_stacks_from_player_inventory(ctx, player_entity_id, &coin_deposit, false) {
            return Err(format!(
                "You need {{0}} hex coins between your inventory and your treasury to evict this tenant.|~{}",
                initial_amount
            ));
        }
        Ok(())
    }

    pub fn collect_paid_rent(&mut self, ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
        // collect coins
        let amount = self.paid_rent;

        if amount == 0 {
            return Ok(());
        }

        let mut player_wallet = unwrap_or_err!(InventoryState::get_player_wallet(ctx, player_entity_id), "Player has no inventory");

        let coins = ItemStack::hex_coins(amount as i32);

        if !player_wallet.add(ctx, coins) {
            return Err("You don't have enough room to collect the hex coins.".into());
        }
        ctx.db.inventory_state().entity_id().update(player_wallet);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.white_list.clear();
        // daily rent is not cleared, we want to keep the same amount as default
        self.active = false;
        self.paid_rent = 0;
        self.defaulted = false;
        self.eviction_timestamp = None;
    }
}
