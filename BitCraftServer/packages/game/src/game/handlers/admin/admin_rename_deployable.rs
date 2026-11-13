use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, components::deployable_state_v2},
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_rename_deployable(ctx: &ReducerContext, deployable_name: String, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let name_lower = deployable_name.to_lowercase();
    let entity_id = unwrap_or_err!(
        ctx.db
            .deployable_state_v2()
            .iter().find(|d| d.nickname.to_lowercase() == name_lower),
        "Deployable not found"
    )
    .entity_id;

    admin_rename_deployable_entity(ctx, entity_id, new_name)
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn admin_rename_deployable_entity(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let mut deployable = unwrap_or_err!(ctx.db.deployable_state_v2().entity_id().find(entity_id), "Deployable not found");
    deployable.nickname = new_name;
    ctx.db.deployable_state_v2().entity_id().update(deployable);

    Ok(())
}
