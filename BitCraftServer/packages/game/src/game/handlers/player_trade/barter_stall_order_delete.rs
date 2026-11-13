use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self, game_state_filters},
    messages::{action_request::PlayerBarterStallOrderDeleteRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn barter_stall_order_delete(ctx: &ReducerContext, request: PlayerBarterStallOrderDeleteRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.shop_entity_id, request.trade_order_entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, shop_entity_id: u64, trade_order_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    if ThreatState::in_combat(ctx, entity_id) {
        return Err("Cannot modify a trade order while in combat".into());
    }

    let coordinates = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap().coordinates();

    if let Some(building) = ctx.db.building_state().entity_id().find(shop_entity_id) {
        if building.distance_to(ctx, &coordinates) > 5 {
            return Err("Too far".into());
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
    }

    let trade_order = unwrap_or_err!(
        ctx.db.trade_order_state().entity_id().find(trade_order_entity_id),
        "Trade order does not exist"
    );

    if trade_order.shop_entity_id != shop_entity_id {
        return Err("Trade order is posted elsewhere.".into());
    }

    ctx.db.trade_order_state().entity_id().delete(trade_order_entity_id);

    Ok(())
}
