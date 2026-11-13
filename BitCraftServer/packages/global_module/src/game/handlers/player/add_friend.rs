use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{
        components::*,
        global::{friends_state, FriendsState},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn add_friend(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    unwrap_or_err!(ctx.db.player_username_state().entity_id().find(player_entity_id), "Invalid player");

    if ctx
        .db
        .friends_state()
        .owner_friend_entity_id()
        .filter((actor_id, player_entity_id))
        .count()
        > 0
    {
        return Err("Already a friend".into());
    }

    ctx.db.friends_state().insert(FriendsState {
        entity_id: game_state::create_entity(ctx),
        owner_entity_id: actor_id,
        friend_entity_id: player_entity_id,
        is_favorite: false,
    });

    Ok(())
}
