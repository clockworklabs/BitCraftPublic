use spacetimedb::{ReducerContext};
use std::collections::HashSet;

use crate::{
    game::{dimensions, game_state::game_state_filters, handlers::player_trade::trade_decline::trade_cancel_server},
    inventory_state, parameters_desc, trade_session_state, unwrap_or_err, InventoryState, TradeSessionState, TradeSessionStatus,
};

impl TradeSessionState {
    pub fn validate_distance(&self, ctx: &ReducerContext) -> Result<(), String> {
        let overworld_max = ctx.db.parameters_desc().version().find(0).unwrap().max_trade_distance_large_tiles;
        let c1 = game_state_filters::coordinates_float(ctx, self.initiator_entity_id).parent_large_tile();
        let c2 = game_state_filters::coordinates_float(ctx, self.acceptor_entity_id).parent_large_tile();
        let max = if c1.dimension == dimensions::OVERWORLD && c2.dimension == dimensions::OVERWORLD {
            overworld_max
        } else {
            1
        };
        if c1.distance_to(c2) > max {
            // schedule a cancellation and fail the current transaction
            spacetimedb::volatile_nonatomic_schedule_immediate!(trade_cancel_server(self.entity_id, "Too far".into()));
            return Err("Other player is too far away to trade".into());
        }
        Ok(())
    }

    pub fn cancel_session_and_update(mut self, ctx: &ReducerContext) -> Result<(), String> {
        let mut inventories_updated = HashSet::new();

        for trade_pocket in self.initiator_offer.iter() {
            //empty
            if trade_pocket.inventory_index < 0 {
                continue;
            }
            let initiator_inventory = unwrap_or_err!(
                InventoryState::get_by_owner_with_index(ctx, self.initiator_entity_id, trade_pocket.inventory_index),
                "Initiator has no inventory"
            );
            inventories_updated.insert(initiator_inventory.entity_id);
        }

        for trade_pocket in self.acceptor_offer.iter() {
            //empty
            if trade_pocket.inventory_index < 0 {
                continue;
            }

            let acceptor_inventory = unwrap_or_err!(
                InventoryState::get_by_owner_with_index(ctx, self.acceptor_entity_id, trade_pocket.inventory_index),
                "Acceptor has no inventory"
            );
            inventories_updated.insert(acceptor_inventory.entity_id);
        }

        for inventory_entity_id in inventories_updated {
            if let Some(mut inventory) = ctx.db.inventory_state().entity_id().find(inventory_entity_id) {
                inventory.unlock_all_pockets();
                ctx.db.inventory_state().entity_id().update(inventory);
            }
        }

        self.status = TradeSessionStatus::SessionResolved;
        self.updated_at = ctx.timestamp;

        ctx.db.trade_session_state().entity_id().update(self);

        Ok(())
    }
}
