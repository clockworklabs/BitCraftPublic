use spacetimedb::{log, ReducerContext, Table};

use crate::game::handlers::authentication::has_role;
use crate::messages::authentication::Role;
use crate::messages::generic::{config, Config};

#[spacetimedb::reducer]
pub fn load_config(ctx: &ReducerContext, environment_names: Vec<String>, contents: Vec<String>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    ctx.db.config().version().delete(0);
    let expected_env = "local"; //DAB Note: I'm Derek and I say we need to replace this once we can access STDB instance name
    let index = environment_names
        .iter()
        .position(|e| e == expected_env)
        .unwrap_or(environment_names.iter().position(|e| e == "local").unwrap_or_default());
    let config = &contents[index];

    let cfg = json::parse(config).unwrap();
    if ctx
        .db
        .config()
        .try_insert(Config {
            version: 0,
            env: cfg["env"].to_string(),
            agents_enabled: true,
        })
        .is_err()
    {
        log::error!("Failed to insert config");
        return Err("Failed to insert config".into());
    }

    log::info!("--- {} config loaded", environment_names[index]);
    Ok(())
}
