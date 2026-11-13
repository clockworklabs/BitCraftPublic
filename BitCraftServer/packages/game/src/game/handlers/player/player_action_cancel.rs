use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{game::game_state, messages::components::*, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_action_cancel(ctx: &ReducerContext, client_cancel: bool) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, client_cancel)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, client_cancel: bool) -> Result<(), String> {
    let mut player_action_state = unwrap_or_err!(
        PlayerActionState::get_state(ctx, &entity_id, &PlayerActionLayer::Base),
        "Missing player action state"
    );

    if player_action_state.action_type == PlayerActionType::Climb || player_action_state.action_type == PlayerActionType::UseElevator {
        // Don't cancel a climb or elevator usage. For now.
        return Ok(());
    }

    if player_action_state.action_type == PlayerActionType::None {
        // Action completed by the time the request reached the server. It's not an error.
        return Ok(());
    }

    player_action_state.last_action_result = PlayerActionResult::Cancel;
    player_action_state.client_cancel = client_cancel;

    ctx.db.player_action_state().auto_id().update(player_action_state);

    Ok(())
}
