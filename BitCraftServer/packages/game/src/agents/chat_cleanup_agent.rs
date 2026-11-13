use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::{game_state, handlers::authentication::has_role},
    messages::{
        authentication::{Role, ServerIdentity},
        components::chat_message_state,
    },
};

#[spacetimedb::table(accessor = chat_cleanup_timer, scheduled(chat_cleanup_agent_loop, at = scheduled_at))]
pub struct ChatCleanupTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = SECONDS_IN_A_DAY;
    let mut count = 0;
    for mut timer in ctx.db.chat_cleanup_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_secs(tick_length).into();
        ctx.db.chat_cleanup_timer().scheduled_id().update(timer);
        log::info!("chat cleanup agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one chat cleanup agents running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = 3600; // 1 hour
    ctx.db
        .chat_cleanup_timer()
        .try_insert(ChatCleanupTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_secs(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn chat_cleanup_agent_insert(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }
    init(ctx);
    Ok(())
}

#[spacetimedb::reducer]
fn chat_cleanup_agent_loop(ctx: &ReducerContext, _timer: ChatCleanupTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to chat cleanup agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let two_days_ago = ctx
        .timestamp
        .checked_sub_duration(Duration::from_secs(2 * SECONDS_IN_A_DAY))
        .unwrap();
    let delete_threshold = game_state::unix(two_days_ago);

    let mut count = 0;
    for chat_message in ctx.db.chat_message_state().iter() {
        if chat_message.timestamp <= delete_threshold {
            count += 1;
            ctx.db.chat_message_state().entity_id().delete(chat_message.entity_id);
        }
    }

    log::info!("Chat Cleanup Agent deleted {count} messages");
}
