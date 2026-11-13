use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self},
    messages::global::blocked_player_state,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn unblock_player(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    if ctx
        .db
        .blocked_player_state()
        .owner_blocked_entity_id()
        .delete((actor_id, player_entity_id))
        == 0
    {
        return Err("Not blocked".into());
    }

    Ok(())
}
