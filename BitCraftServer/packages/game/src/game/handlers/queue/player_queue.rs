use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{signed_in_player_state, user_previous_region_state, user_state, UserState},
        generic::RegionSignInParameters,
        queue::{player_queue_state, PlayerQueueState},
    },
    unwrap_or_err, unwrap_or_return,
};

use super::end_grace_period::{end_grace_period_timer, EndGracePeriodTimer, GracePeriodType};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_queue_join(ctx: &ReducerContext) -> Result<(), String> {
    let user_state = unwrap_or_err!(
        ctx.db.user_state().identity().find(ctx.sender()),
        "You must have a character to join the queue"
    );

    let region_sign_in_parameters = unwrap_or_err!(RegionSignInParameters::get(ctx), "Failed to get RegionSignInParameters");
    if region_sign_in_parameters.is_signing_in_blocked && !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Server is unavailable at this time, please try again later.".to_string());
    }

    if user_state.can_sign_in {
        EndGracePeriodTimer::new(ctx, ctx.sender(), GracePeriodType::SignIn);
        return Ok(());
    }

    //Rejoin the queue in case of a client crash / alt + f4
    if let Some(end_grace_period_timer) = ctx.db.end_grace_period_timer().identity().find(ctx.sender()) {
        ctx.db
            .end_grace_period_timer()
            .scheduled_id()
            .delete(end_grace_period_timer.scheduled_id);
        return Ok(());
    }

    if has_role(ctx, &ctx.sender(), Role::SkipQueue) {
        allow_sign_in(ctx, user_state);
        return Ok(());
    }

    let region_sign_in_parameters = unwrap_or_err!(RegionSignInParameters::get(ctx), "Failed to get RegionSignInParameters");
    let queued_player_count = ctx.db.player_queue_state().count();

    if queued_player_count == 0 && get_signed_in_player_count(ctx) < region_sign_in_parameters.max_signed_in_players {
        allow_sign_in(ctx, user_state);
        return Ok(());
    }

    if queued_player_count >= region_sign_in_parameters.max_queue_length {
        return Err("The queue is full, please try again at another time".into());
    }

    enqueue(ctx, user_state.entity_id);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_queue_leave(ctx: &ReducerContext) -> Result<(), String> {
    if let Some(user_state) = ctx.db.user_state().identity().find(ctx.sender()) {
        dequeue(ctx, user_state.entity_id);
    }

    Ok(())
}

fn get_signed_in_player_count(ctx: &ReducerContext) -> u64 {
    let signed_in_player_count = ctx.db.signed_in_player_state().count();
    let grace_period_player_count = ctx
        .db
        .end_grace_period_timer()
        .iter()
        .filter(|x| x.grace_period_type == GracePeriodType::SignIn)
        .count() as u64;

    signed_in_player_count + grace_period_player_count
}

fn enqueue(ctx: &ReducerContext, entity_id: u64) {
    let _ = ctx.db.player_queue_state().try_insert(PlayerQueueState { index: 0, entity_id });
}

pub fn allow_sign_in(ctx: &ReducerContext, mut user_state: UserState) {
    let identity = user_state.identity;

    //When this expires, it will processes the queue and set can_sign_in back to false
    EndGracePeriodTimer::new(ctx, user_state.identity, GracePeriodType::SignIn);

    user_state.can_sign_in = true;
    ctx.db.user_state().entity_id().update(user_state);

    ctx.db.user_previous_region_state().identity().delete(identity);
}

pub fn process_queue(ctx: &ReducerContext) {
    //Subtract 1 as the EndGracePeriodTimer record isn't deleted yet when this fires
    let signed_in_player_count = get_signed_in_player_count(ctx) as i64 - 1;
    let region_sign_in_parameters = unwrap_or_return!(RegionSignInParameters::get(ctx), "Failed to get RegionSignInParameters");
    let max_signed_in_players = region_sign_in_parameters.max_signed_in_players as i64;
    let free_slot_count = max_signed_in_players - signed_in_player_count;

    if free_slot_count < 1 {
        return;
    }

    let mut queued_players: Vec<PlayerQueueState> = ctx.db.player_queue_state().iter().collect();
    queued_players.sort_by_key(|x| x.index);

    // usize is 32 bits on wasm32, so saturate rather than truncate a very large slot count.
    let free_slot_count = usize::try_from(free_slot_count).unwrap_or(usize::MAX);
    for queued_player in queued_players.into_iter().take(free_slot_count) {
        if let Some(user_state) = ctx.db.user_state().entity_id().find(queued_player.entity_id) {
            //Ensure the QueueJoin grace period is deleted before inserting a SignIn grace period
            ctx.db.end_grace_period_timer().identity().delete(user_state.identity);
            allow_sign_in(ctx, user_state);
        }

        ctx.db.player_queue_state().index().delete(queued_player.index);
    }
}

pub fn dequeue(ctx: &ReducerContext, entity_id: u64) {
    ctx.db.player_queue_state().entity_id().delete(entity_id);
}
