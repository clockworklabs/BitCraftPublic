use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{game::game_state, messages::action_request::PlayerPausePlayTimerRequest, player_state, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn pause_play_timer(ctx: &ReducerContext, request: PlayerPausePlayTimerRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let mut player = unwrap_or_err!(ctx.db.player_state().entity_id().find(actor_id), "Invalid player");

    if request.paused {
        if player.session_start_timestamp != 0 {
            let time_played = game_state::unix(ctx.timestamp) - player.session_start_timestamp;
            player.time_played += time_played;
            player.session_start_timestamp = 0;
            ctx.db.player_state().entity_id().update(player);
        }
    } else {
        if player.session_start_timestamp == 0 {
            player.session_start_timestamp = game_state::unix(ctx.timestamp);
            ctx.db.player_state().entity_id().update(player);
        }
    }
    Ok(())
}
