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
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_post_buy_order(ctx: &ReducerContext, request: PlayerPostOrderRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
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

    if request.coins_spent != unwrap_or_err!(request.max_unit_price.checked_mul(request.quantity), "overflow") {
        return Err("Coins don't match the offer".into());
    }

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    let mut coins_spent = 0;

    let mut required_item = ItemStack::new(ctx, request.item_id, request.item_type, request.quantity);
    let available_coins = request.coins_spent;

    // Find all sell_orders matching the price and item id, and collect items from those, sorted by increasing price (with timestamp for tie-breaking)
    resolve_buy_order(
        ctx,
        actor_id,
        &mut required_item,
        claim_entity_id,
        request.max_unit_price,
        available_coins,
        &mut coins_spent,
    );

    // available_coins => either quantity x unitary cost or alloted budget for multi-sell-order purchases (10x1 + 5x2)
    // coins spent => coins spent fulfilling orders, can be less than available coins

    let savings = if request.persist_order {
        // this is a normal persisted buy order, with a single unit price for all items.
        // savings => 100 items at 10 coins each for a total of 1000; 60 were sold at 4 coins (coins spent 240 instead of 600), 40 on sale at 10 each
        // coins spent should be 640 instead of 1000 (savings of 360)
        //                 (100              - 40                    ) * 10            - 240 = 600 - 240 = 360
        (request.quantity - required_item.quantity) * request.max_unit_price - coins_spent
    } else {
        // this is a 10x1 + 5x2 order. All coins are normally spent (unless there was a last-second quantity change on the server)
        // in which case there can be a savings (available coin > coins spent). If funds are lacking an error will be returned later in this function.
        available_coins - coins_spent
    };

    // Buy Order => Remove coins used to create buy order from inventory or surrounding storages. Spend the total of coins no matter whether it sells or not.
    let required_coins = vec![ItemStack::hex_coins(available_coins)];
    InventoryState::withdraw_full_durability_from_player_inventory_and_nearby_deployables(ctx, actor_id, &required_coins, |x| {
        building.distance_to(ctx, &x)
    })?;

    // Create a closed listing for the buyer coins saved in the transaction
    if savings > 0 {
        let refunded_coins = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            claim_entity_id,
            item_stack: ItemStack::hex_coins(savings),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(refunded_coins);
    }

    // Post the remaining buy order
    if required_item.quantity > 0 {
        if !request.persist_order {
            return Err("Items are no longer available for this price".into());
        }

        if available_coins < 0 || coins_spent < 0 {
            return Err("Integer overflow".into());
        }

        let buy_order = AuctionListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            claim_entity_id,
            item_id: required_item.item_id,
            item_type: required_item.item_type as i32,
            price_threshold: request.max_unit_price,
            quantity: required_item.quantity,
            timestamp: ctx.timestamp,
            stored_coins: required_item.quantity * request.max_unit_price, // unsold portion of the order
        };
        ctx.db.buy_order_state().insert(buy_order);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

pub fn resolve_buy_order(
    ctx: &ReducerContext,
    owner_entity_id: u64,
    required_item: &mut ItemStack,
    claim_entity_id: u64,
    price_threshold: i32,
    available_coins: i32,
    coins_spent: &mut i32,
) {
    let item_id = required_item.item_id;
    let item_type = required_item.item_type;

    let mut matching_orders: Vec<AuctionListingState> = ctx
        .db
        .sell_order_state()
        .item()
        .filter((item_id, item_type as i32))
        .filter(|order| order.price_threshold <= price_threshold)
        .collect();
    matching_orders.sort_by(|a, b| a.price_threshold.cmp(&b.price_threshold).then(a.timestamp.cmp(&b.timestamp)));

    // Create ClosedListingStates for sell orders that have been fulfilled for both parties
    for i in 0..matching_orders.len() {
        let remaining_budget = available_coins - *coins_spent;
        let quantity_sold = matching_orders[i]
            .quantity
            .min(remaining_budget / matching_orders[i].price_threshold)
            .min(required_item.quantity);
        if quantity_sold <= 0 {
            break;
        }

        let matching_order = matching_orders.get_mut(i).unwrap();

        required_item.quantity -= quantity_sold;
        *coins_spent = coins_spent
            .checked_add(
                quantity_sold
                    .checked_mul(matching_order.price_threshold)
                    .expect("coins_spent integer overflow"),
            )
            .expect("coins_spent integer overflow");

        let buyer_items = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id,
            claim_entity_id,
            item_stack: ItemStack::new(ctx, required_item.item_id, required_item.item_type, quantity_sold),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(buyer_items);

        let seller_coins = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: matching_order.owner_entity_id,
            claim_entity_id: matching_order.claim_entity_id,
            item_stack: ItemStack::hex_coins(matching_order.price_threshold * quantity_sold),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(seller_coins);

        matching_order.quantity -= quantity_sold;

        ctx.db.market_trade_event().insert(MarketTradeEvent {
            claim_entity_id: matching_order.claim_entity_id,
            listing_entity_id: matching_order.entity_id,
            listing_type: MarketOrderType::SellOrder,
            buyer_entity_id: owner_entity_id,
            seller_entity_id: matching_order.owner_entity_id,
            item_id: required_item.item_id,
            item_type: required_item.item_type,
            quantity: quantity_sold,
            unit_price: matching_order.price_threshold,
            listing_remaining_quantity: matching_order.quantity,
            timestamp: ctx.timestamp,
        });

        if matching_order.quantity > 0 {
            ctx.db.sell_order_state().entity_id().update(matching_order.clone());
        } else {
            ctx.db.sell_order_state().entity_id().delete(matching_order.entity_id);
        }
        if required_item.quantity <= 0 {
            break;
        }
    }
}
