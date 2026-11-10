use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, reducer_helpers::player_action_helpers},
    messages::{
        action_request::PlayerPostOrderRequest,
        components::{
            buy_order_state, closed_listing_state, sell_order_state, AuctionListingState, ClosedListingState, HealthState, InventoryState,
        },
        events::{market_trade_event, MarketOrderType, MarketTradeEvent},
        game_util::{ItemStack, ItemType},
    },
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_post_sell_order(ctx: &ReducerContext, request: PlayerPostOrderRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if request.item_id == ItemStack::hex_coins(1).item_id && request.item_type == ItemType::Item {
        return Err("Invalid order".into());
    }

    if request.quantity <= 0 {
        return Err("Invalid quantity".into());
    }

    if request.max_unit_price <= 0 {
        return Err("Invalid coins".into());
    }

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    // Remove required items from inventory or surrounding storages
    let mut required_item = ItemStack::new(ctx, request.item_id, request.item_type, request.quantity);
    let required_items = vec![required_item];
    InventoryState::withdraw_full_durability_from_player_inventory_and_nearby_deployables(ctx, actor_id, &required_items, |x| {
        building.distance_to(ctx, &x)
    })?;

    resolve_sell_order(ctx, actor_id, &mut required_item, claim_entity_id, request.max_unit_price);

    // An instant sale must roll back if a concurrent fill removed the requested liquidity.
    if required_item.quantity > 0 && !request.persist_order {
        return Err("Orders are no longer available for this price".into());
    }

    // Post the remaining sell order
    if required_item.quantity > 0 && request.persist_order {
        let sell_order = AuctionListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            claim_entity_id,
            item_id: required_item.item_id,
            item_type: required_item.item_type as i32,
            price_threshold: request.max_unit_price,
            quantity: required_item.quantity,
            timestamp: ctx.timestamp,
            stored_coins: 0,
        };
        ctx.db.sell_order_state().insert(sell_order);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

pub fn resolve_sell_order(
    ctx: &ReducerContext,
    owner_entity_id: u64,
    required_item: &mut ItemStack,
    claim_entity_id: u64,
    price_threshold: i32,
) {
    let item_id = required_item.item_id;
    let item_type = required_item.item_type;

    // Find all buy_orders matching the price and item id, and collect coins from those, sorted by decreasing price (with timestamp for tie-breaking)
    let mut matching_orders: Vec<AuctionListingState> = ctx
        .db
        .buy_order_state()
        .item()
        .filter((item_id, item_type as i32))
        .filter(|order| order.price_threshold >= price_threshold)
        .collect();
    matching_orders.sort_by(|a, b| b.price_threshold.cmp(&a.price_threshold).then(a.timestamp.cmp(&b.timestamp)));

    // Create ClosedListingStates for sell orders that have been fulfilled for both parties
    for i in 0..matching_orders.len() {
        let quantity_sold = matching_orders[i].quantity.min(required_item.quantity);

        let matching_order = matching_orders.get_mut(i).unwrap();

        required_item.quantity -= quantity_sold;

        let buyer_items = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: matching_order.owner_entity_id,
            claim_entity_id: matching_order.claim_entity_id,
            item_stack: ItemStack::new(ctx, required_item.item_id, required_item.item_type, quantity_sold),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(buyer_items);

        let sale_coins_amount = matching_order
            .price_threshold
            .checked_mul(quantity_sold)
            .expect("sale_coins_amount integer overflow");

        let seller_coins = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id,
            claim_entity_id,
            item_stack: ItemStack::hex_coins(sale_coins_amount),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(seller_coins);

        matching_order.quantity -= quantity_sold;
        matching_order.stored_coins -= sale_coins_amount;

        ctx.db.market_trade_event().insert(MarketTradeEvent {
            claim_entity_id: matching_order.claim_entity_id,
            listing_entity_id: matching_order.entity_id,
            listing_type: MarketOrderType::BuyOrder,
            buyer_entity_id: matching_order.owner_entity_id,
            seller_entity_id: owner_entity_id,
            item_id: required_item.item_id,
            item_type: required_item.item_type,
            quantity: quantity_sold,
            unit_price: matching_order.price_threshold,
            listing_remaining_quantity: matching_order.quantity,
            timestamp: ctx.timestamp,
        });

        if matching_order.quantity > 0 {
            ctx.db.buy_order_state().entity_id().update(matching_order.clone());
        } else {
            if matching_order.stored_coins > 0 {
                // Grant the owner of the closed buy order the amount of coins left unspent
                let left_over_coins = ClosedListingState {
                    entity_id: game_state::create_entity(ctx),
                    owner_entity_id: matching_order.owner_entity_id,
                    claim_entity_id: matching_order.claim_entity_id,
                    item_stack: ItemStack::hex_coins(matching_order.stored_coins),
                    timestamp: ctx.timestamp,
                };
                ctx.db.closed_listing_state().insert(left_over_coins);
            }
            ctx.db.buy_order_state().entity_id().delete(matching_order.entity_id);
        }

        if required_item.quantity <= 0 {
            break;
        }
    }
}
