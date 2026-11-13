use bitcraft_macro::feature_gate;
use crate::messages::components::*;
use crate::{game::game_state, messages::action_request::PlayerSetDefaultDeployableRequest};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_set_default_deployable(ctx: &ReducerContext, request: PlayerSetDefaultDeployableRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut player_prefs = ctx.db.player_prefs_state().entity_id().find(actor_id).unwrap();
    player_prefs.default_deployable_collectible_id = request.deployable_collectible_id;
    ctx.db.player_prefs_state().entity_id().update(player_prefs);

    Ok(())
}
