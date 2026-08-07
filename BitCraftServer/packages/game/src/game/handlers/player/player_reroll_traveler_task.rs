use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        game_state,
        reducer_helpers::player_action_helpers,
    },
    messages::components::{traveler_task_state, HealthState, PlayerTimestampState, TravelerTaskState},
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_reroll_traveler_task(ctx: &ReducerContext, task_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let task = ctx
        .db
        .traveler_task_state()
        .entity_id()
        .find(task_entity_id)
        .ok_or_else(|| "This task no longer exists".to_string())?;

    if task.player_entity_id != actor_id {
        return Err("This task is not yours to reroll".into());
    }

    if task.completed {
        return Err("This task has already been completed".into());
    }

    TravelerTaskState::reroll_task(ctx, task_entity_id)?;
    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);
    Ok(())
}
