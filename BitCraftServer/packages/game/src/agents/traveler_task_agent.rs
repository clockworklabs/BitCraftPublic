use spacetimedb::{log, ReducerContext, ScheduleAt, Table};

use crate::{
    agents,
    game::{game_state, handlers::authentication::has_role, reducer_helpers::timer_helpers::now_plus_secs},
    messages::{
        authentication::{Role, ServerIdentity},
        components::{signed_in_player_state, traveler_task_credit_state, TravelerTaskCreditState},
    },
};

pub const TRAVELER_TASK_RESET_INTERVAL_SECS: i32 = 7 * 24 * 60 * 60;

#[spacetimedb::table(name = traveler_task_loop_timer, public, scheduled(traveler_task_agent_loop, at = scheduled_at))]
pub struct TravelerTaskLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

fn get_latest_timer(ctx: &ReducerContext) -> Option<TravelerTaskLoopTimer> {
    ctx.db.traveler_task_loop_timer().iter().max_by_key(|x| x.scheduled_id)
}

pub fn last_tick(ctx: &ReducerContext) -> i32 {
    next_tick(ctx) - TRAVELER_TASK_RESET_INTERVAL_SECS
}

pub fn next_tick(ctx: &ReducerContext) -> i32 {
    if let Some(timer) = get_latest_timer(ctx) {
        if let ScheduleAt::Time(timestamp) = timer.scheduled_at {
            return game_state::unix(timestamp);
        }
    }

    game_state::unix(ctx.timestamp) + TRAVELER_TASK_RESET_INTERVAL_SECS
}

pub fn schedule_next_tick(ctx: &ReducerContext) {
    ctx.db
        .traveler_task_loop_timer()
        .try_insert(TravelerTaskLoopTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(TRAVELER_TASK_RESET_INTERVAL_SECS as u64, ctx.timestamp),
        })
        .ok()
        .unwrap();
}

pub fn init(ctx: &ReducerContext) {
    // Schedule instantly an agent tick - we need the ctx to come from a server call rather than the spacetimedb::init() call
    ctx.db
        .traveler_task_loop_timer()
        .try_insert(TravelerTaskLoopTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(0, ctx.timestamp),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn traveler_task_agent_loop(ctx: &ReducerContext, _timer: TravelerTaskLoopTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        log::error!("Unauthorized access to traveler task agent");
        return;
    }

    schedule_next_tick(ctx);

    if !agents::should_run(ctx) {
        return;
    }

    let mut count = 0;
    for signed_in_player in ctx.db.signed_in_player_state().iter() {
        if TravelerTaskCreditState::refresh_player_credits(ctx, signed_in_player.entity_id) {
            count += 1;
        }
    }

    log::info!("Refreshed task credits for {} players", count);
}

#[spacetimedb::reducer]
fn admin_reset_traveler_task_credits(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    for row in ctx.db.traveler_task_loop_timer().iter() {
        ctx.db.traveler_task_loop_timer().delete(row);
    }

    for mut row in ctx.db.traveler_task_credit_state().iter() {
        row.last_reset = 0;
        ctx.db.traveler_task_credit_state().entity_id().update(row);
    }

    init(ctx);

    Ok(())
}
