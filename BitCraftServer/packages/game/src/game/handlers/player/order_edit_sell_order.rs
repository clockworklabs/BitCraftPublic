use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, reducer_helpers::player_action_helpers},
    messages::{
        action_request::PlayerEditOrderRequest,
        components::{closed_listing_state, sell_order_state, AuctionListingState, ClosedListingState, HealthState, InventoryState},
        game_util::{ItemStack, ItemType},
    },
    unwrap_or_err,
};

use super::order_post_sell_order;

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_edit_sell_order(ctx: &ReducerContext, request: PlayerEditOrderRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if request.quantity <= 0 {
        return Err("Invalid quantity".into());
    }
    if request.coins <= 0 {
        return Err("Invalid coins".into());
    }

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    let mut sell_order = unwrap_or_err!(
        ctx.db.sell_order_state().entity_id().find(request.order_entity_id),
        "Sell order no longer exists"
    );

    if sell_order.owner_entity_id != actor_id {
        return Err("You don't own this sell order".into());
    }

    let new_threshold = request.coins;
    let new_quantity = request.quantity;
    let process_order = new_threshold < sell_order.price_threshold;
    if sell_order.claim_entity_id != claim_entity_id {
        return Err("You cannot edit this listing from there".into());
    }

    if new_quantity == sell_order.quantity && new_threshold == sell_order.price_threshold {
        return Err("No change".into());
    }

    let required_items_delta = new_quantity - sell_order.quantity;
    if required_items_delta > 0 {
        // Expend more items
        let required_items = vec![ItemStack::new(
            ctx,
            sell_order.item_id,
            ItemType::to_enum(sell_order.item_type),
            required_items_delta,
        )];
        InventoryState::withdraw_full_durability_from_player_inventory_and_nearby_deployables(ctx, actor_id, &required_items, |x| {
            building.distance_to(ctx, &x)
        })?;
    } else if required_items_delta < 0 {
        // Refund some items
        let refunded_items = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            claim_entity_id,
            item_stack: ItemStack::new(
                ctx,
                sell_order.item_id,
                ItemType::to_enum(sell_order.item_type),
                required_items_delta.abs(),
            ),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(refunded_items);
    }

    sell_order.price_threshold = new_threshold;
    sell_order.quantity = new_quantity;

    // Only retry matching when the sell price decreases.
    let mut required_item = ItemStack::new(
        ctx,
        sell_order.item_id,
        ItemType::to_enum(sell_order.item_type),
        sell_order.quantity,
    );
    if process_order {
        order_post_sell_order::resolve_sell_order(ctx, actor_id, &mut required_item, claim_entity_id, sell_order.price_threshold);
    }

    if required_item.quantity > 0 {
        sell_order.quantity = required_item.quantity;
        ctx.db.sell_order_state().entity_id().update(sell_order);
    } else {
        ctx.db.sell_order_state().entity_id().delete(sell_order.entity_id);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
