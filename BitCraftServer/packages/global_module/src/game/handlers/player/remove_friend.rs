use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self},
    messages::global::friends_state,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn remove_friend(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    if ctx.db.friends_state().owner_friend_entity_id().delete((actor_id, player_entity_id)) == 0 {
        return Err("Not a friend".into());
    }

    Ok(())
}
