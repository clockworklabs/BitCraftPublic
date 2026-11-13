use spacetimedb::{duration, log, ReducerContext, Table};

use crate::{
    agents,
    messages::{authentication::ServerIdentity, components::*},
};

#[spacetimedb::table(accessor = trade_session_loop_timer, scheduled(trade_sessions_agent_loop, at = scheduled_at))]
pub struct TradeSessionLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .trade_session_loop_timer()
        .try_insert(TradeSessionLoopTimer {
            scheduled_id: 0,
            scheduled_at: duration!(5s).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn trade_sessions_agent_loop(ctx: &ReducerContext, _timer: TradeSessionLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to trade_sessions agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let now = ctx.timestamp;
    let expiration_duration = 45; // 45 seconds

    let expired_and_resolved_sessions = ctx.db.trade_session_state().iter().filter_map(|session| {
        //Session concluded
        if session.status == TradeSessionStatus::SessionResolved {
            return Some(session);
        }

        //Timeout
        let d = now.duration_since(session.updated_at);
        if let Some(duration) = d {
            let duration = duration.as_secs() as i32;
            if duration >= expiration_duration {
                return Some(session);
            }
        }

        //Check if players logged out
        if ctx
            .db
            .signed_in_player_state()
            .entity_id()
            .find(session.initiator_entity_id)
            .is_none()
        {
            return Some(session);
        }
        if ctx
            .db
            .signed_in_player_state()
            .entity_id()
            .find(session.acceptor_entity_id)
            .is_none()
        {
            return Some(session);
        }
        None
    });

    for mut session in expired_and_resolved_sessions {
        if session.status != TradeSessionStatus::SessionResolved {
            session.resolution_message = "Trade timed out".into();
            session.cancel_session_and_update(ctx).unwrap();
        } else {
            let session_entity_id = session.entity_id;
            ctx.db.trade_session_state().entity_id().delete(session_entity_id);
        }
    }
}
