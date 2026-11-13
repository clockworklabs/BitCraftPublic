use crate::{game::handlers::authentication::has_role, messages::authentication::Role, unwrap_or_err, user_state, vault_state};
use spacetimedb::{log, Identity, ReducerContext};
use std::str::FromStr;

#[spacetimedb::reducer]
pub fn admin_remove_collectible(ctx: &ReducerContext, identity: String, collectible_id: i32, quantity: u32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = Identity::from_str(identity.as_str()).map_err(|_| "Identity couldn't be parsed".to_string())?;
    let user = unwrap_or_err!(ctx.db.user_state().identity().find(identity), "User not found");
    let mut vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(user.entity_id), "Vault not found");

    let removed = vault.remove_collectible_quantity(collectible_id, quantity);
    if removed > 0 {
        ctx.db.vault_state().entity_id().update(vault);
    }

    log::info!(
        "Removed {} collectible(s) with id {} from identity {})",
        removed,
        collectible_id,
        identity,
    );

    Ok(())
}
