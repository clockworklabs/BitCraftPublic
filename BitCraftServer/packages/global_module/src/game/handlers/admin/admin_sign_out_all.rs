use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::{admin::admin_broadcast, authentication::has_role},
    inter_module::sign_player_out,
    messages::authentication::Role,
    signed_in_player_state, user_state,
};

#[spacetimedb::reducer]
pub fn admin_sign_out_all(ctx: &ReducerContext, region: u8) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    admin_broadcast::reduce(
        ctx,
        region,
        "You were signed out".into(),
        "You were signed out. The server may be undergoing maintenance.".into(),
        true,
    );

    if region == 0 {
        for player in ctx.db.signed_in_player_state().iter() {
            if let Some(user) = ctx.db.user_state().entity_id().find(player.entity_id) {
                let _ = sign_player_out::send_message(ctx, user.identity);
            }
        }
    }

    log::info!(
        "admin_sign_out_all(): Authorized : Completed sign_out process for {} users as Admin {}",
        ctx.db.user_state().iter().count(),
        ctx.sender().to_hex()
    );
    Ok(())
}
