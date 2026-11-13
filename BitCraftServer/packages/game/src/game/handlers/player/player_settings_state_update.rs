use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::messages::components::{player_settings_state, PlayerSettingsState};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_settings_state_update(ctx: &ReducerContext, player_settings_state: PlayerSettingsState) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    if player_settings_state.entity_id != actor_id {
        return Err("Invalid player".into());
    }

    ctx.db.player_settings_state().entity_id().insert_or_update(player_settings_state);

    Ok(())
}
