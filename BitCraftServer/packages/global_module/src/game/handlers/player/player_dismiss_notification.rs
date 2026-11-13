use bitcraft_macro::feature_gate;
use crate::{game::game_state, messages::global::player_developer_notification_state};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_dismiss_notification(ctx: &ReducerContext) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    ctx.db.player_developer_notification_state().entity_id().delete(actor_id);
    Ok(())
}
