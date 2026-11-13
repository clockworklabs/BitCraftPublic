use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerTradeDeclineSessionRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_decline_session(ctx: &ReducerContext, request: PlayerTradeDeclineSessionRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.session_entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, session_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    let mut trade_session = unwrap_or_err!(
        ctx.db.trade_session_state().entity_id().find(session_entity_id),
        "No such trade session."
    );

    if trade_session.status != TradeSessionStatus::SessionOffered {
        return Err("Cannot decline this session.".into());
    }

    if trade_session.acceptor_entity_id != entity_id {
        return Err("Cannot decline trade if not recipient.".into());
    }

    trade_session.status = TradeSessionStatus::SessionResolved;
    trade_session.updated_at = ctx.timestamp;
    trade_session.resolution_message = "Trade declined".into();

    ctx.db.trade_session_state().entity_id().update(trade_session);

    Ok(())
}
