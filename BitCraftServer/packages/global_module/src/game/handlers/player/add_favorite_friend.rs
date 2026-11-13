use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self},
    messages::global::friends_state,
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn add_favorite_friend(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let mut friends_state = unwrap_or_err!(
        ctx.db
            .friends_state()
            .owner_friend_entity_id()
            .filter((actor_id, player_entity_id))
            .next(),
        "Not a friend"
    );

    friends_state.is_favorite = true;

    ctx.db.friends_state().entity_id().update(friends_state);

    Ok(())
}
