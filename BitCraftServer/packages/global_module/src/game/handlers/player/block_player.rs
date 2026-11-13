use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{
        components::*,
        global::{blocked_player_state, BlockedPlayerState},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn block_player(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    unwrap_or_err!(ctx.db.player_username_state().entity_id().find(player_entity_id), "Invalid player");

    if ctx
        .db
        .blocked_player_state()
        .owner_blocked_entity_id()
        .filter((actor_id, player_entity_id))
        .count()
        > 0
    {
        return Err("Already blocked".into());
    }

    ctx.db.blocked_player_state().insert(BlockedPlayerState {
        owner_entity_id: actor_id,
        blocked_entity_id: player_entity_id,
    });

    Ok(())
}
