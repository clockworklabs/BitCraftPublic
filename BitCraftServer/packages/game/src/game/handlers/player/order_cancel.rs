use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{
        action_request::PlayerOrderCancelRequest,
        components::{buy_order_state, sell_order_state, AuctionListingState, HealthState},
    },
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_cancel(ctx: &ReducerContext, request: PlayerOrderCancelRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    if let Some(order) = ctx.db.sell_order_state().entity_id().find(request.auction_listing_entity_id) {
        if order.owner_entity_id != actor_id {
            return Err("You are not the owner of this listing".into());
        }
        if order.claim_entity_id != claim_entity_id {
            return Err("You cannot cancel this listing from there".into());
        }

        // Refund "sold" items
        order.cancel_sell_order(ctx);
    } else if let Some(order) = ctx.db.buy_order_state().entity_id().find(request.auction_listing_entity_id) {
        if order.owner_entity_id != actor_id {
            return Err("You are not the owner of this listing".into());
        }
        if order.claim_entity_id != claim_entity_id {
            return Err("You cannot cancel this listing from there".into());
        }
        // Refund coins
        order.cancel_buy_order(ctx);
    }

    Ok(())
}
