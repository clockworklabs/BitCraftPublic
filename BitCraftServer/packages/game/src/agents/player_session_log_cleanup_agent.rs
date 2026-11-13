use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::game_state,
    messages::{authentication::ServerIdentity, region::player_session_log},
};

// A little more than the 4 weeks the CS portal looks back over
const DELETE_SESSIONS_AFTER_DAYS: i32 = 30;

#[spacetimedb::table(accessor = player_session_log_cleanup_loop_timer, scheduled(player_session_log_cleanup_loop, at = scheduled_at))]
pub struct PlayerSessionLogCleanupLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    if ctx.db.player_session_log_cleanup_loop_timer().iter().count() == 0 {
        ctx.db
            .player_session_log_cleanup_loop_timer()
            .insert(PlayerSessionLogCleanupLoopTimer {
                scheduled_id: 0,
                scheduled_at: spacetimedb::ScheduleAt::Time(ctx.timestamp),
            });
    }
}

pub fn init(ctx: &ReducerContext) {
    update_timer(ctx);
}

#[spacetimedb::reducer]
fn player_session_log_cleanup_loop(ctx: &ReducerContext, _timer: PlayerSessionLogCleanupLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to player session log cleanup agent");
        return;
    }

    ctx.db
        .player_session_log_cleanup_loop_timer()
        .insert(PlayerSessionLogCleanupLoopTimer {
            scheduled_id: 0,
            scheduled_at: spacetimedb::ScheduleAt::Time(ctx.timestamp + Duration::from_secs(60 * 60)),
        });

    if !agents::should_run(ctx) {
        return;
    }

    let cutoff = game_state::unix(ctx.timestamp) - DELETE_SESSIONS_AFTER_DAYS * 24 * 60 * 60;
    let deleted = ctx.db.player_session_log().sign_out_timestamp().delete(..cutoff);
    if deleted > 0 {
        log::info!("Player session log cleanup agent deleted {deleted} sessions");
    }
}
