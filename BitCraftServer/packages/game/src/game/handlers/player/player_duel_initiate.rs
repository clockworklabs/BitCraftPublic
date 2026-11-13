use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{
        components::{duel_state, HealthState, PlayerTimestampState},
        region::*,
    },
};

#[spacetimedb::reducer]
#[feature_gate("combat")]
pub fn player_duel_initiate(ctx: &ReducerContext, target_player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if ctx.db.duel_state().initiator_entity_id().find(actor_id).is_some() {
        return Err("You are already duelling".into());
    }
    if ctx.db.duel_state().acceptor_entity_id().find(actor_id).is_some() {
        return Err("You are already duelling".into());
    }
    if ctx.db.duel_state().initiator_entity_id().find(target_player_entity_id).is_some() {
        return Err("That player is already duelling".into());
    }
    if ctx.db.duel_state().acceptor_entity_id().find(target_player_entity_id).is_some() {
        return Err("That player is already duelling".into());
    }

    // Leave [30] seconds to answer the vote.
    // Todo: put that as a parameter somewhere.
    PlayerVoteState::insert_with_end_timer(
        ctx,
        PlayerVoteType::Duel,
        actor_id,
        vec![actor_id, target_player_entity_id],
        true,
        1.0,
        30.0,
        0,
        0,
    );

    Ok(())
}
