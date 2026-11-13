use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerTradeDeclineRequest, authentication::ServerIdentity, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_decline(ctx: &ReducerContext, request: PlayerTradeDeclineRequest) -> Result<(), String> {
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

    if trade_session.status == TradeSessionStatus::SessionOffered || trade_session.status == TradeSessionStatus::SessionResolved {
        return Err("Cannot decline this trade.".into());
    }

    if entity_id != trade_session.acceptor_entity_id && entity_id != trade_session.initiator_entity_id {
        return Err("Not a member of trade session.".into());
    }

    trade_session.resolution_message = "Trade canceled".into();
    trade_session.cancel_session_and_update(ctx)
}

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_cancel_server(ctx: &ReducerContext, session_entity_id: u64, resolution_message: String) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let mut trade_session = unwrap_or_err!(
        ctx.db.trade_session_state().entity_id().find(session_entity_id),
        "No such trade session."
    );
    trade_session.resolution_message = resolution_message;
    trade_session.cancel_session_and_update(ctx)
}
