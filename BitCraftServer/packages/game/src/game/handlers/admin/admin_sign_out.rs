use std::str::FromStr;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, Identity, ReducerContext, Table};

use crate::{
    game::handlers::{authentication::has_role, player::sign_out::sign_out_internal},
    messages::{authentication::Role, components::user_state, events::{player_signed_out_event, PlayerSignedOutEvent, PlayerSignedOutReason}},
};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_sign_out(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    if let Some(mut user_state) = ctx.db.user_state().identity().find(identity) {
        user_state.can_sign_in = false;
        ctx.db.user_state().entity_id().update(user_state);
    }

    sign_out_internal(ctx, identity, false);
    ctx.db.player_signed_out_event().insert(PlayerSignedOutEvent {
        identity,
        reason: PlayerSignedOutReason::AdminAction,
    });

    log::info!(
        "admin_sign_out(): Authorized : Completed sign_out process for User {} as Admin {}",
        identity.to_hex(),
        ctx.sender().to_hex()
    );

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_sign_out_string(ctx: &ReducerContext, identity: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let identity = Identity::from_str(identity.as_str());
    if identity.is_err() {
        return Err("Identity couldn't be parsed".into());
    }
    let identity = identity.unwrap();

    admin_sign_out(ctx, identity)
}
