use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, reducer_helpers::player_action_helpers},
    messages::{
        action_request::PlayerEditOrderRequest,
        components::{buy_order_state, closed_listing_state, AuctionListingState, ClosedListingState, HealthState, InventoryState},
        game_util::{ItemStack, ItemType},
    },
    unwrap_or_err,
};

use super::order_post_buy_order;

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_edit_buy_order(ctx: &ReducerContext, request: PlayerEditOrderRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if request.quantity <= 0 {
        return Err("Invalid quantity".into());
    }
    if request.coins <= 0 {
        return Err("Invalid coins".into());
    }

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    let mut buy_order = unwrap_or_err!(
        ctx.db.buy_order_state().entity_id().find(request.order_entity_id),
        "Buy order no longer exists"
    );

    if buy_order.owner_entity_id != actor_id {
        return Err("You don't own this buy order".into());
    }

    let new_threshold = request.coins;
    let new_quantity = request.quantity;
    let process_order = new_threshold > buy_order.price_threshold;
    if buy_order.claim_entity_id != claim_entity_id {
        return Err("You cannot edit this listing from there".into());
    }

    if new_quantity == buy_order.quantity && new_threshold == buy_order.price_threshold {
        return Err("No change".into());
    }

    let mut required_coins_delta = 0;

    // Adjust all existing purchases with new price threshold
    required_coins_delta += buy_order
        .quantity
        .checked_mul(new_threshold - buy_order.price_threshold)
        .expect("required_coins_delta integer overflow");

    // Adjust coins for purchasing a different amount of items
    required_coins_delta = required_coins_delta
        .checked_add(
            (new_quantity - buy_order.quantity)
                .checked_mul(new_threshold)
                .expect("required_coins_delta multiplication integer overflow"),
        )
        .expect("required_coins_delta adddition integer overflow");

    buy_order.price_threshold = new_threshold;
    buy_order.quantity = new_quantity;
    buy_order.stored_coins += required_coins_delta;
    if buy_order.stored_coins < 0 {
        return Err("Overflow detected. Change is not allowed".into());
    }

    // Only retry matching when the buy price increases.
    let mut required_item = ItemStack::new(ctx, buy_order.item_id, ItemType::to_enum(buy_order.item_type), buy_order.quantity);
    let mut coins_spent = 0;
    if process_order {
        order_post_buy_order::resolve_buy_order(
            ctx,
            actor_id,
            &mut required_item,
            claim_entity_id,
            request.coins,
            buy_order.stored_coins,
            &mut coins_spent,
        );

        // coins spent => fulfilling orders.
        // total_coins_requested => for placing the order
        // savings => 100 items at 10 coins each for a total of 1000; 60 were sold at 4 coins (coins spent 240 instead of 600), 40 on sale at 10 each
        // coins spent should be 640 instead of 1000 (savings of 360)
        //                 (100                - 40                    ) * 10                        - 240 = 600 - 240 = 360
        let savings = (buy_order.quantity - required_item.quantity) * buy_order.price_threshold - coins_spent;
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
            // Savings have been paid out; remove them from stored_coins so a
            // subsequent cancel cannot refund them a second time.
            buy_order.stored_coins = buy_order.stored_coins.checked_sub(savings).unwrap();
        }
    }

    if required_item.quantity > 0 {
        buy_order.quantity = required_item.quantity;
        buy_order.stored_coins -= coins_spent;
        ctx.db.buy_order_state().entity_id().update(buy_order);
    } else {
        ctx.db.buy_order_state().entity_id().delete(buy_order.entity_id);
    }

    if required_coins_delta > 0 {
        let required_coins = vec![ItemStack::hex_coins(required_coins_delta)];
        InventoryState::withdraw_full_durability_from_player_inventory_and_nearby_deployables(ctx, actor_id, &required_coins, |x| {
            building.distance_to(ctx, &x)
        })?;
    } else if required_coins_delta < 0 {
        // Grant coins
        let refunded_coins = ClosedListingState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            claim_entity_id,
            item_stack: ItemStack::hex_coins(required_coins_delta.abs()),
            timestamp: ctx.timestamp,
        };
        ctx.db.closed_listing_state().insert(refunded_coins);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
