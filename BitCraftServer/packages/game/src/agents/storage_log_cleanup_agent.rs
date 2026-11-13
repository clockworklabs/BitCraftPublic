use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::game_state,
    messages::{authentication::ServerIdentity, components::storage_log_state},
};

const DELETE_LOGS_AFTER_DAYS: i32 = 14;

#[spacetimedb::table(accessor = storage_log_cleanup_loop_timer, scheduled(storage_log_cleanup_loop, at = scheduled_at))]
pub struct StorageLogCleanupLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    if ctx.db.storage_log_cleanup_loop_timer().iter().count() == 0 {
        ctx.db.storage_log_cleanup_loop_timer().insert(StorageLogCleanupLoopTimer {
            scheduled_id: 0,
            scheduled_at: spacetimedb::ScheduleAt::Time(ctx.timestamp),
        });
    }
}

pub fn init(ctx: &ReducerContext) {
    update_timer(ctx);
}

#[spacetimedb::reducer]
fn storage_log_cleanup_loop(ctx: &ReducerContext, _timer: StorageLogCleanupLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to starving agent");
        return;
    }

    ctx.db.storage_log_cleanup_loop_timer().insert(StorageLogCleanupLoopTimer {
        scheduled_id: 0,
        scheduled_at: spacetimedb::ScheduleAt::Time(ctx.timestamp + Duration::from_secs(60 * 60 * 24)),
    });

    if !agents::should_run(ctx) {
        return;
    }

    let now = game_state::days_since_unix_epoch(ctx.timestamp);
    //Delete logs that just expired...
    ctx.db
        .storage_log_state()
        .days_since_epoch()
        .delete(now - DELETE_LOGS_AFTER_DAYS - 1);
    //... as well as a few extra days in case agent was disabled
    ctx.db
        .storage_log_state()
        .days_since_epoch()
        .delete(now - DELETE_LOGS_AFTER_DAYS - 2);
    ctx.db
        .storage_log_state()
        .days_since_epoch()
        .delete(now - DELETE_LOGS_AFTER_DAYS - 3);
}
