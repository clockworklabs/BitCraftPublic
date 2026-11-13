use crate::{deployable_state_v2, game::reducer_helpers::deployable_helpers, messages::authentication::ServerIdentity};
use spacetimedb::{log, ReducerContext};

#[spacetimedb::table(accessor = hide_deployable_timer, scheduled(hide_deployable, at = scheduled_at))]
pub struct HideDeployableTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    #[unique]
    pub entity_id: u64,
}

#[spacetimedb::reducer]
pub fn hide_deployable(ctx: &ReducerContext, timer: HideDeployableTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to hide_deployable");
        return;
    }

    if let Some(mut deployable) = ctx.db.deployable_state_v2().entity_id().find(timer.entity_id) {
        deployable.hidden = true;
        ctx.db.deployable_state_v2().entity_id().update(deployable);

        //Disambark all passengers
        deployable_helpers::expel_passengers(ctx, timer.entity_id, false, false);
    } else {
        log::error!(
            "Hide deployable timer could not find deployable with entity_id: {}",
            timer.entity_id
        );
    }
}
