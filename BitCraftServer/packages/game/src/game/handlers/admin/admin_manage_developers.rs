use std::str::FromStr;

use spacetimedb::{Identity, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::{developer, Developer, Role},
        components::user_state,
    },
};

#[spacetimedb::reducer]
pub fn insert_developer_identity(
    ctx: &ReducerContext,
    identity: String,
    developer_name: String,
    service_name: String,
    email: String,
    is_external: bool,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = Identity::from_str(&identity).unwrap();
    if let Some(user_state) = ctx.db.user_state().identity().find(identity) {
        return Err(format!(
            "The provided identity is already in use for player entity_id: {}. You muse provide a new identity.",
            user_state.entity_id
        )
        .into());
    }
    if let Some(developer) = ctx.db.developer().identity().find(identity) {
        return Err(format!(
            "The provided identity is already in use for developer: {}.",
            developer.developer_name
        )
        .into());
    }

    ctx.db.developer().insert(Developer {
        identity,
        developer_name,
        service_name,
        email,
        is_external,
    });

    Ok(())
}

#[spacetimedb::reducer]
pub fn delete_developer_identity(ctx: &ReducerContext, identity: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = Identity::from_str(&identity).map_err(|e| format!("Invalid identity: {e}"))?;

    match ctx.db.developer().identity().find(identity) {
        Some(developer) => {
            ctx.db.developer().delete(developer);
            Ok(())
        }
        None => Err("Developer not found".to_string()),
    }
}
