use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{
        components::{
            building_state, buy_order_state, closed_listing_state, sell_order_state, AuctionListingState, BuildingState, ClosedListingState,
        },
        game_util::{ItemStack, ItemType},
        static_data::{building_desc, BuildingCategory},
    },
};

impl AuctionListingState {
    pub fn marketplace_for_action(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
        max_distance: i32,
    ) -> Result<BuildingState, String> {
        let building = ctx
            .db
            .building_state()
            .entity_id()
            .find(building_entity_id)
            .ok_or("Building does not exist")?;
        if building.claim_entity_id == 0 {
            return Err("Invalid marketplace claim".into());
        }
        if building.distance_to(ctx, &game_state::game_state_filters::coordinates_float(ctx, actor_id).into()) > max_distance {
            return Err("Too far".into());
        }

        let desc = ctx
            .db
            .building_desc()
            .id()
            .find(building.building_description_id)
            .ok_or("Building description does not exist")?;
        if !desc.has_category(ctx, BuildingCategory::TownMarket) {
            return Err("This building is not a marketplace".into());
        }

        Ok(building)
    }

    pub fn cancel_sell_order(&self, ctx: &ReducerContext) {
        let refunded_items = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: self.owner_entity_id,
            claim_entity_id: self.claim_entity_id,
            item_stack: ItemStack::new(ctx, self.item_id, ItemType::to_enum(self.item_type), self.quantity),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(refunded_items);
        ctx.db.sell_order_state().entity_id().delete(self.entity_id);
    }

    pub fn cancel_buy_order(&self, ctx: &ReducerContext) {
        // Refund coins
        let refunded_items = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: self.owner_entity_id,
            claim_entity_id: self.claim_entity_id,
            item_stack: ItemStack::hex_coins(self.stored_coins),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(refunded_items);
        ctx.db.buy_order_state().entity_id().delete(self.entity_id);
    }
}
