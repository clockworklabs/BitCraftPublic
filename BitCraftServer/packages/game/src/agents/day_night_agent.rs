use spacetimedb::{duration, log, ReducerContext, Table};

use crate::{
    agents,
    game::{game_state, reducer_helpers::timer_helpers::now_plus_millis},
    messages::authentication::ServerIdentity,
    parameters_desc,
};

#[spacetimedb::table(accessor = day_night_loop_timer, scheduled(day_night_agent_loop, at = scheduled_at))]
pub struct DayNightLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .day_night_loop_timer()
        .try_insert(DayNightLoopTimer {
            scheduled_id: 0,
            scheduled_at: duration!(0s).into(), // schedule right away
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn day_night_agent_loop(ctx: &ReducerContext, _timer: DayNightLoopTimer) {
    if !agents::should_run(ctx) {
        return;
    }

    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to day_night agent");
        return;
    }

    if is_day_time(ctx) {
        day_tick(ctx)
    } else {
        night_tick(ctx);
    }
}

pub fn day_tick(ctx: &ReducerContext) {
    // Note: This agent does nothing for now.
    // Here is where you'll insert events happening at day break.

    let time = time_of_day(ctx);
    let night_fall_time = ((day_duration(ctx) - time) * 1000) as u64;
    ctx.db
        .day_night_loop_timer()
        .try_insert(DayNightLoopTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_millis(night_fall_time, ctx.timestamp),
        })
        .ok()
        .unwrap();
}

pub fn night_tick(ctx: &ReducerContext) {
    // Note: This agent does nothing for now.
    // Here is where you'll insert events happening at night fall.

    let time = time_of_day(ctx);
    let day_break_time = ((day_duration(ctx) + night_duration(ctx) - time) * 1000) as u64;
    ctx.db
        .day_night_loop_timer()
        .try_insert(DayNightLoopTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_millis(day_break_time, ctx.timestamp),
        })
        .ok()
        .unwrap();
}

pub fn night_duration(ctx: &ReducerContext) -> i32 {
    ctx.db.parameters_desc().version().find(0).unwrap().nighttime
}

pub fn day_duration(ctx: &ReducerContext) -> i32 {
    ctx.db.parameters_desc().version().find(0).unwrap().daytime
}

pub fn time_of_day(ctx: &ReducerContext) -> i32 {
    game_state::unix(ctx.timestamp) % (night_duration(ctx) + day_duration(ctx))
}

pub fn is_night_time(ctx: &ReducerContext) -> bool {
    time_of_day(ctx) >= day_duration(ctx)
}

pub fn is_day_time(ctx: &ReducerContext) -> bool {
    !is_night_time(ctx)
}
