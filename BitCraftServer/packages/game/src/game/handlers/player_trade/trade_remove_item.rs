use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{entities::building_state::InventoryState, game_state},
    messages::{
        action_request::PlayerTradeRemoveItemRequest,
        components::*,
        game_util::{ItemStack, TradePocket},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_remove_item(ctx: &ReducerContext, request: PlayerTradeRemoveItemRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.session_entity_id, request.pocket_index as usize)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, session_entity_id: u64, pocket_index: usize) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    let mut trade_session = unwrap_or_err!(
        ctx.db.trade_session_state().entity_id().find(session_entity_id),
        "No such trade session."
    );

    if trade_session.status == TradeSessionStatus::SessionOffered || trade_session.status == TradeSessionStatus::SessionResolved {
        return Err("Cannot add item to this session.".into());
    }

    if entity_id != trade_session.acceptor_entity_id && entity_id != trade_session.initiator_entity_id {
        return Err("Not a member of trade session.".into());
    }

    trade_session.validate_distance(ctx)?;

    let current_inventory_pocket: i32;
    if entity_id == trade_session.initiator_entity_id {
        current_inventory_pocket = trade_session.initiator_offer[pocket_index].inventory_pocket_index;
        if current_inventory_pocket < 0 {
            return Err("Pocket is empty".into());
        }

        trade_session.initiator_offer[pocket_index] = TradePocket {
            inventory_index: -1,
            inventory_pocket_index: -1,
            contents: ItemStack::empty(),
        }
    } else {
        current_inventory_pocket = trade_session.acceptor_offer[pocket_index].inventory_pocket_index;
        if current_inventory_pocket < 0 {
            return Err("Pocket is empty".into());
        }

        trade_session.acceptor_offer[pocket_index] = TradePocket {
            inventory_index: -1,
            inventory_pocket_index: -1,
            contents: ItemStack::empty(),
        }
    }

    if current_inventory_pocket >= 0 {
        let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, entity_id), "Player has no inventory");
        inventory.unlock_pocket(current_inventory_pocket as usize);
        ctx.db.inventory_state().entity_id().update(inventory);
    }

    trade_session.status = TradeSessionStatus::SessionAccepted;
    trade_session.updated_at = ctx.timestamp;

    ctx.db.trade_session_state().entity_id().update(trade_session);

    Ok(())
}
