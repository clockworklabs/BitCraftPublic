use spacetimedb::{log, ReducerContext};

use crate::messages::{
    authentication::ServerIdentity,
    inter_module::{AdminBroadcastMessageMsg, MessageContentsV5},
};

use super::send_inter_module_message;

#[spacetimedb::table(name = sytem_chat_broadcast_timer, scheduled(system_chat_broadcast_scheduled, at = scheduled_at))]
pub struct SystemChatBroadcastTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub message: String,
}

pub fn send_message(ctx: &ReducerContext, message: String) {
    send_inter_module_message(
        ctx,
        MessageContentsV5::AdminBroadcastMessage(AdminBroadcastMessageMsg {
            title: "".into(),
            message,
            sign_out: false,
        }),
        super::InterModuleDestination::Global,
    );
}

#[spacetimedb::reducer]
fn system_chat_broadcast_scheduled(ctx: &ReducerContext, timer: SystemChatBroadcastTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        log::error!("Unauthorized access to system_broadcast_scheduled");
        return;
    }

    send_message(ctx, timer.message);
}
