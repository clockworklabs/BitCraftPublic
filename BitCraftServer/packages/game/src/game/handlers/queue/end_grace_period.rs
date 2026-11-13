use spacetimedb::{log, Identity, ReducerContext, Table};

use crate::{
    game::reducer_helpers::timer_helpers::now_plus_secs,
    messages::{authentication::ServerIdentity, components::user_state, generic::RegionSignInParameters},
    unwrap_or_return,
};

use super::player_queue;

#[spacetimedb::table(accessor = end_grace_period_timer, scheduled(end_grace_period, at = scheduled_at))]
pub struct EndGracePeriodTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    #[unique]
    pub identity: Identity,
    pub grace_period_type: GracePeriodType,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
pub enum GracePeriodType {
    SignIn,
    QueueJoin,
}

impl EndGracePeriodTimer {
    #[expect(clippy::new_ret_no_self, reason = "pre-existing signature, left as is when clippy was enabled")]
    pub fn new(ctx: &ReducerContext, identity: Identity, grace_period_type: GracePeriodType) {
        let region_sign_in_parameters = unwrap_or_return!(RegionSignInParameters::get(ctx), "Failed to get RegionSignInParameters");

        if let Some(mut existing) = ctx.db.end_grace_period_timer().identity().find(identity) {
            existing.scheduled_at = now_plus_secs(region_sign_in_parameters.grace_period_seconds, ctx.timestamp);
            existing.grace_period_type = grace_period_type;
            ctx.db.end_grace_period_timer().scheduled_id().update(existing);
            return;
        }

        ctx.db.end_grace_period_timer().insert(EndGracePeriodTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(region_sign_in_parameters.grace_period_seconds, ctx.timestamp),
            identity,
            grace_period_type,
        });
    }
}

#[spacetimedb::reducer]
fn end_grace_period(ctx: &ReducerContext, timer: EndGracePeriodTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to end_grace_period");
        return;
    }

    let mut user_state = unwrap_or_return!(
        ctx.db.user_state().identity().find(timer.identity),
        "Identity {} has no UserState",
        timer.identity
    );

    match timer.grace_period_type {
        GracePeriodType::SignIn => {
            player_queue::process_queue(ctx);

            user_state.can_sign_in = false;
            ctx.db.user_state().entity_id().update(user_state);
        }
        GracePeriodType::QueueJoin => player_queue::dequeue(ctx, user_state.entity_id),
    }
}
