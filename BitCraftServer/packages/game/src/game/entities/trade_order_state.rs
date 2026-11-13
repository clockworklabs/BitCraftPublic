use spacetimedb::{log, ReducerContext, Table};

use crate::game::game_state;
use crate::messages::components::TradeOrderState;
use crate::messages::game_util::ItemStack;
use crate::{trade_order_state, InventoryState};

impl TradeOrderState {
    pub const MARKET_MODE_CURRENCY_ID: i32 = 1; //Hex Coins

    pub fn create(
        ctx: &ReducerContext,
        shop_entity_id: u64,
        stock: i32,
        offer_items: &[ItemStack],
        required_items: &[ItemStack],
        traveler_trade_order_id: Option<i32>,
    ) {
        let trade_order_entity_id = game_state::create_entity(ctx);

        // For traveler trade orders, we use TradeOrderState to keep track of the remaining stock only.
        // Offered and requested items are empty, and the values are used directly from static data
        let is_traveler_trade_order = traveler_trade_order_id.is_some();

        let trade_order = TradeOrderState {
            entity_id: trade_order_entity_id,
            shop_entity_id,
            remaining_stock: stock,
            offer_items: if is_traveler_trade_order { Vec::new() } else { offer_items.to_owned() },
            offer_cargo_id: Vec::new(), // # MIGRATION # OBSOLETE 
            required_items: if is_traveler_trade_order {
                Vec::new()
            } else {
                required_items.to_owned()
            },
            required_cargo_id: Vec::new(), // # MIGRATION # OBSOLETE 
            traveler_trade_order_id,
        };

        if ctx.db.trade_order_state().try_insert(trade_order).is_err() {
            log::error!("Failed to insert trade order");
        }
    }

    pub fn is_valid_market_mode_order(
        offer_items: &[ItemStack],
        required_items: &[ItemStack],
    ) -> bool {
        if let Some(offered_item) = offer_items.first() {
            if offered_item.item_id == TradeOrderState::MARKET_MODE_CURRENCY_ID {
                return required_items.len() == 1;
            }
        }

        if let Some(required_item) = required_items.first() {
            if required_item.item_id == TradeOrderState::MARKET_MODE_CURRENCY_ID {
                return offer_items.len() == 1;
            }
        }

        false
    }

    pub fn get_num_similar_market_mode_orders(
        trade_orders: &[TradeOrderState],
        offer_items: &[ItemStack],
        required_items: &[ItemStack],
    ) -> usize {
        if let Some(offered_item) = offer_items.first() {
            if offered_item.item_id != TradeOrderState::MARKET_MODE_CURRENCY_ID {
                return trade_orders
                    .iter()
                    .filter(|x| x.offer_items.iter().any(|x| x.item_id == offered_item.item_id))
                    .count();
            }
        }

        if let Some(required_item) = required_items.first() {
            if required_item.item_id != TradeOrderState::MARKET_MODE_CURRENCY_ID {
                return trade_orders
                    .iter()
                    .filter(|x| x.required_items.iter().any(|x| x.item_id == required_item.item_id))
                    .count();
            }
        }

        0
    }

    pub fn has_any_in_stock_trade_orders(ctx: &ReducerContext, shop_entity_id: u64) -> bool {
        let trade_orders: Vec<TradeOrderState> = ctx.db.trade_order_state().shop_entity_id().filter(shop_entity_id).collect();

        if trade_orders.is_empty() {
            return false;
        }

        let Some(inventory_state) = InventoryState::get_by_owner(ctx, shop_entity_id) else {
            return false;
        };

        trade_orders.iter().any(|x| {
            x.remaining_stock > 0 && inventory_state.has(&x.offer_items)
        })
    }
}
