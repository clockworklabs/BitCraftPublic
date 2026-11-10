use bitcraft_macro::feature_gate;
use crate::{
    game::{game_state, handlers::player::player_vote_conclude::PlayerVoteConcludeTimer, reducer_helpers::timer_helpers::now_plus_secs},
    messages::{
        action_request::PlayerVoteAnswerRequest,
        global::{player_vote_state, PlayerVoteAnswer},
    },
    unwrap_or_err,
};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use super::player_vote_conclude::player_vote_conclude_timer;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_vote_answer(ctx: &ReducerContext, request: PlayerVoteAnswerRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    reduce(ctx, request.vote_entity_id, actor_id, request.accept)
}

pub fn reduce(ctx: &ReducerContext, vote_entity_id: u64, player_entity_id: u64, accept: bool) -> Result<(), String> {
    let mut vote = unwrap_or_err!(ctx.db.player_vote_state().entity_id().find(&vote_entity_id), "Query is over.").clone();

    if vote.outcome != PlayerVoteAnswer::None {
        return Err("Query is over.".into());
    }

    let pass_threshold = vote.pass_threshold;

    let index = unwrap_or_err!(
        vote.participants_entity_id.iter().position(|p| *p == player_entity_id),
        "Not part of this query"
    );

    vote.answers[index] = if accept { PlayerVoteAnswer::Yes } else { PlayerVoteAnswer::No };

    let yes_count = vote.answers.iter().filter(|a| **a == PlayerVoteAnswer::Yes).count();
    let no_count = vote.answers.iter().filter(|a| **a == PlayerVoteAnswer::No).count();
    let total_count = vote.answers.iter().count();
    let success = yes_count as f32 / total_count as f32 >= pass_threshold;
    let failure = no_count as f32 / total_count as f32 > 1.0 - pass_threshold;
    let outcome = if success {
        PlayerVoteAnswer::Yes
    } else if failure {
        PlayerVoteAnswer::No
    } else {
        PlayerVoteAnswer::None
    };
    vote.outcome = outcome;

    // if outcome is determined, conclude the vote after a small delay that will let the clients use the vote information before it's deleted
    if outcome != PlayerVoteAnswer::None {
        vote.play_outcome(ctx)?;
        ctx.db
            .player_vote_conclude_timer()
            .try_insert(PlayerVoteConcludeTimer {
                scheduled_id: 0,
                scheduled_at: now_plus_secs(1, ctx.timestamp),
                vote_entity_id,
            })
            .ok()
            .unwrap();
    }
    ctx.db.player_vote_state().entity_id().update(vote);

    Ok(())
}
