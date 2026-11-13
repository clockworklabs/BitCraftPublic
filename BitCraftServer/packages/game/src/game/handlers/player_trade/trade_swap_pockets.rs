use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerTradeSwapPocketsRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_swap_pockets(ctx: &ReducerContext, request: PlayerTradeSwapPocketsRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(
        ctx,
        actor_id,
        request.session_entity_id,
        request.from_index as usize,
        request.to_index as usize,
    )
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, session_entity_id: u64, from_index: usize, to_index: usize) -> Result<(), String> {
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
    let max_pocket_index = trade_session.initiator_offer.len() - 1; // Cannot swap the cargo since it's the only pocket

    if from_index >= max_pocket_index {
        return Err("Pocket index out of range".into());
    }
    if to_index >= max_pocket_index {
        return Err("Pocket index out of range".into());
    }

    if entity_id == trade_session.initiator_entity_id {
        let temp = trade_session.initiator_offer[from_index].clone();
        trade_session.initiator_offer[from_index] = trade_session.initiator_offer[to_index].clone();
        trade_session.initiator_offer[to_index] = temp;
    } else {
        let temp = trade_session.acceptor_offer[from_index].clone();
        trade_session.acceptor_offer[from_index] = trade_session.acceptor_offer[to_index].clone();
        trade_session.acceptor_offer[to_index] = temp;
    }

    trade_session.status = TradeSessionStatus::SessionAccepted;
    trade_session.updated_at = ctx.timestamp;

    ctx.db.trade_session_state().entity_id().update(trade_session);

    Ok(())
}
