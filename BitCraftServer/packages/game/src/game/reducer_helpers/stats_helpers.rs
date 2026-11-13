use crate::messages::authentication::ServerIdentity;
use crate::messages::components::PlayerState;
use spacetimedb::ReducerContext;

#[spacetimedb::table(accessor = collect_stats_timer, scheduled(collect_stats_reducer, at = scheduled_at))]
pub struct CollectStatsTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
}

#[spacetimedb::reducer]
pub fn collect_stats_reducer(ctx: &ReducerContext, timer: CollectStatsTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    // Delayed collect stats, usually after a buff expiration.
    // No harm in doing an extra collect_stats at original expiration if buff duration is increased
    PlayerState::collect_stats(ctx, timer.entity_id);
    Ok(())
}
