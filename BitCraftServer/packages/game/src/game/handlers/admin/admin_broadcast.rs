use spacetimedb::ReducerContext;

use crate::{
    game::handlers::authentication::has_role,
    inter_module::send_inter_module_message,
    messages::authentication::Role,
};

#[spacetimedb::reducer]
pub fn admin_broadcast_msg_region(ctx: &ReducerContext, title: String, message: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }
    reduce(ctx, title, message, false);
    Ok(())
}

pub fn reduce(ctx: &ReducerContext, title: String, message: String, sign_out: bool) {
    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::AdminBroadcastMessage(
            crate::messages::inter_module::AdminBroadcastMessageMsg { title, message, sign_out },
        ),
        crate::inter_module::InterModuleDestination::Global,
    );
}
