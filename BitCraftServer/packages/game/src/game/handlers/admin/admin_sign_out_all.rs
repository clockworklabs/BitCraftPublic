use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::{admin::admin_broadcast, authentication::has_role, player::sign_out::sign_out_internal},
    messages::authentication::Role,
    signed_in_player_state, user_state,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_sign_out_all_region(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    admin_broadcast::reduce(
        ctx,
        "You were signed out".into(),
        "You were signed out. The server may be undergoing maintenance.".into(),
        true,
    );

    for player in ctx.db.signed_in_player_state().iter() {
        if let Some(user) = ctx.db.user_state().entity_id().find(player.entity_id) {
            sign_out_internal(ctx, user.identity, false);
        }
    }

    log::info!(
        "admin_sign_out_all(): Authorized : Completed sign_out process for {} users as Admin {}",
        ctx.db.user_state().iter().count(),
        ctx.sender().to_hex()
    );
    Ok(())
}
