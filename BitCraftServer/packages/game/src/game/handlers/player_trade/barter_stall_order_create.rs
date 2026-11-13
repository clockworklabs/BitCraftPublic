use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    building_desc, deployable_desc,
    game::{
        discovery::Discovery,
        game_state::{self, game_state_filters},
    },
    messages::{
        action_request::PlayerBarterStallOrderCreateRequest,
        components::*,
        game_util::{ItemStack, ItemType},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn barter_stall_order_create(ctx: &ReducerContext, request: PlayerBarterStallOrderCreateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(
        ctx,
        actor_id,
        request.shop_entity_id,
        request.remaining_stock,
        &request.offer_items,
        &request.required_items,
    )
}

pub fn reduce(
    ctx: &ReducerContext,
    entity_id: u64,
    shop_entity_id: u64,
    stock: i32,
    offer_items: &Vec<ItemStack>,
    required_items: &Vec<ItemStack>,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    if offer_items.iter().any(|i| i.quantity < 0) || required_items.iter().any(|i| i.quantity < 0) {
        return Err("Invalid request.".into());
    }

    if ThreatState::in_combat(ctx, entity_id) {
        return Err("Cannot execute a trade order while in combat".into());
    }

    let barter_stall_state = unwrap_or_err!(
        ctx.db.barter_stall_state().entity_id().find(shop_entity_id),
        "Unknown barter stall"
    );

    let coordinates = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap().coordinates();

    if let Some(building) = ctx.db.building_state().entity_id().find(shop_entity_id) {
        if building.distance_to(ctx, &coordinates) > 5 {
            return Err("Too far".into());
        }
        let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
        if let Some(function) = building_desc.functions.iter().find(|f| f.trade_orders > 0) {
            let max_orders = function.trade_orders;
            let order_count = ctx.db.trade_order_state().shop_entity_id().filter(building.entity_id).count();
            if order_count as i32 >= max_orders {
                return Err("Maximum amount of trade orders reached".into());
            }
        } else {
            return Err("Invalid building type".into());
        }
        game_state_filters::validate_barter_permissions(ctx, entity_id, &building, coordinates.dimension)?;
    } else if let Some(deployable) = ctx.db.deployable_state_v2().entity_id().find(shop_entity_id) {
        let deployable_location = ctx
            .db
            .mobile_entity_state()
            .entity_id()
            .find(shop_entity_id)
            .unwrap()
            .coordinates();
        if deployable_location.distance_to(coordinates) > 5 {
            return Err("Too far".into());
        }
        if deployable.owner_id != entity_id {
            return Err("Only the deployable owner can edit listings".into());
        }
        let deployable_desc = ctx.db.deployable_desc().id().find(deployable.deployable_description_id).unwrap();
        if deployable_desc.barter == 0 {
            return Err("Invalid deployable type".into());
        }
        let order_count = ctx.db.trade_order_state().shop_entity_id().filter(shop_entity_id).count();
        if order_count as i32 >= deployable_desc.barter {
            return Err("Maximum amount of trade orders reached".into());
        }
    }

    for item in offer_items {
        if item.item_type == ItemType::Item
            && !Discovery::already_discovered_item(ctx, entity_id, item.item_id) {
                return Err("Cannot offer an item you didn't discover first.".into());
            }
        if item.item_type == ItemType::Cargo
            && !Discovery::already_discovered_cargo(ctx, entity_id, item.item_id) {
                return Err("Cannot offer a cargo you didn't discover first.".into());
            }
    }

    for item in required_items {
        if item.item_type == ItemType::Item
            && !Discovery::already_discovered_item(ctx, entity_id, item.item_id) {
                return Err("Cannot request an item you didn't discover first.".into());
            }
        if item.item_type == ItemType::Cargo
            && !Discovery::already_discovered_cargo(ctx, entity_id, item.item_id) {
                return Err("Cannot request a cargo you didn't discover first.".into());
            }
    }

    if barter_stall_state.market_mode_enabled {
        if !TradeOrderState::is_valid_market_mode_order(offer_items, required_items) {
            return Err("May only list for currency in market mode".into());
        }

        if TradeOrderState::get_num_similar_market_mode_orders(
            &ctx.db.trade_order_state().shop_entity_id().filter(shop_entity_id).collect::<Vec<_>>(),
            offer_items,
            required_items,
        ) > 0
        {
            return Err("May only list an item once in market mode".into());
        }
    }

    TradeOrderState::create(ctx, shop_entity_id, stock, offer_items, required_items, None);

    Ok(())
}
