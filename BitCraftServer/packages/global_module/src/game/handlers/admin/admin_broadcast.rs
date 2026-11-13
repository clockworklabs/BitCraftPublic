use spacetimedb::ReducerContext;

use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, generic::admin_broadcast},
};

#[spacetimedb::reducer]
pub fn admin_broadcast_msg(ctx: &ReducerContext, region: u8, title: String, message: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }
    if region != 0 {
        return Err("Region admin broadcasts must be sent to the region module directly".into());
    }
    reduce(ctx, region, title, message, false);
    Ok(())
}

pub fn reduce(ctx: &ReducerContext, _region: u8, title: String, message: String, sign_out: bool) {
    let mut broadcast = ctx.db.admin_broadcast().version().find(0).unwrap();
    broadcast.title = title;
    broadcast.message = message;
    broadcast.sign_out = sign_out;
    broadcast.timestamp = ctx.timestamp;
    ctx.db.admin_broadcast().version().update(broadcast);
}
