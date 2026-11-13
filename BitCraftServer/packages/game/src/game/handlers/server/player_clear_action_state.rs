use spacetimedb::ReducerContext;

use crate::{
    messages::{authentication::ServerIdentity, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn player_clear_action_state(
    ctx: &ReducerContext,
    actor_id: u64,
    current_action: PlayerActionType,
    layer: PlayerActionLayer,
    last_action_result: PlayerActionResult,
) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, current_action, layer, last_action_result)
}

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    current_action: PlayerActionType,
    layer: PlayerActionLayer,
    last_action_result: PlayerActionResult,
) -> Result<(), String> {
    let mut player_action = unwrap_or_err!(PlayerActionState::get_state(ctx, &actor_id, &layer), "Player doesn't exist");
    player_action.last_action_result = last_action_result;
    player_action.action_type = current_action;
    ctx.db.player_action_state().auto_id().update(player_action);
    Ok(())
}
