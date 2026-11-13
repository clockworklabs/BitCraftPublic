use bitcraft_macro::shared_table_reducer;
use spacetimedb::{duration, log, ReducerContext, Table, TimeDuration};

use crate::{agents, game::handlers::player::sign_out, messages::{authentication::ServerIdentity, events::{player_signed_out_event, PlayerSignedOutEvent, PlayerSignedOutReason}}, player_timestamp_state, user_state};

#[spacetimedb::table(accessor = auto_logout_loop_timer, scheduled(auto_logout_loop, at = scheduled_at))]
pub struct AutoLogoutLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .auto_logout_loop_timer()
        .try_insert(AutoLogoutLoopTimer {
            scheduled_id: 0,
            scheduled_at: duration!(30s).into(), // 5 min ticks
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn auto_logout_loop(ctx: &ReducerContext, _timer: AutoLogoutLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to auto_logout agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let now = ctx.timestamp;
    let expired = now + (TimeDuration::from_micros(-60 * 30 * 1_000_000)); // no activity since 30 minutes

    for player in ctx.db.player_timestamp_state().iter() {
        if player.timestamp < expired {
            if let Some(user_state) = ctx.db.user_state().entity_id().find(player.entity_id) {
                sign_out::sign_out_internal(ctx, user_state.identity, true);
                ctx.db.player_signed_out_event().insert(PlayerSignedOutEvent {
                    identity: user_state.identity,
                    reason: PlayerSignedOutReason::Inactivity,
                });
            } else {
                log::error!("Signed in player state without a UserState: {}", player.entity_id);
            }
        }
    }
}
