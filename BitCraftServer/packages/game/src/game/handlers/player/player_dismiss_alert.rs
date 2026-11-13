use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerDismissAlertRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_dismiss_alert(ctx: &ReducerContext, request: PlayerDismissAlertRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, request.alert_entity_id)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, alert_entity_id: u64) -> Result<(), String> {
    let alert_state = unwrap_or_err!(ctx.db.alert_state().entity_id().find(alert_entity_id), "Alert no longer exists.");
    if alert_state.player_entity_id != actor_id {
        return Err("Not authorized.".into());
    }
    ctx.db.alert_state().entity_id().delete(alert_entity_id);
    Ok(())
}
