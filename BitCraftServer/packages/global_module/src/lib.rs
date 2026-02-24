pub mod agents;
pub mod game;
pub mod i18n;
pub mod import_global_data;
pub mod import_reducers;
pub mod inter_module;
pub mod macros;
pub mod messages;
pub mod translations;
pub mod utils;

use crate::game::coordinates::*;
use crate::messages::authentication::{IdentityRole, Role, ServerIdentity};
use crate::messages::generic::AdminBroadcast;
use crate::messages::generic::{Config, Globals};
use messages::authentication::identity_role;
use messages::generic::{admin_broadcast, config, globals};
use spacetimedb::{log, ReducerContext, Table};

use crate::messages::components::*;
use crate::messages::static_data::*;

#[spacetimedb::reducer(init)]
pub fn initialize(ctx: &ReducerContext) -> Result<(), String> {
    match ctx.db.config().version().find(&0) {
        Some(config) => {
            if config.env != "dev" {
                // This check is to prevent access to this reducer after the db has been initialized.
                if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
                    return Err("Caller is not the owner of the database".into());
                }
            }
        }
        _ => {}
    };

    if ctx
        .db
        .identity_role()
        .try_insert(IdentityRole {
            role: Role::Admin,
            identity: ctx.sender,
        })
        .is_err()
    {
        log::error!("Failed to insert owner identity");
    }
    if ctx
        .db
        .identity_role()
        .try_insert(IdentityRole {
            role: Role::Admin,
            identity: ctx.identity(),
        })
        .is_err()
    {
        log::error!("Failed to insert database identity");
    }

    ServerIdentity::set(&ctx);

    if ctx
        .db
        .admin_broadcast()
        .try_insert(AdminBroadcast {
            version: 0,
            title: String::new(),
            message: String::new(),
            sign_out: false,
            timestamp: ctx.timestamp,
        })
        .is_err()
    {
        log::error!("Failed to insert AdminBroadcast");
    }

    if ctx
        .db
        .globals()
        .try_insert(Globals {
            version: 0,
            entity_pk_counter: 1, // 0 == overworld dimension description
            dimension_counter: 1,
            region_index: 0,
        })
        .is_err()
    {
        log::error!("Failed to insert globals");
    }

    if ctx
        .db
        .config()
        .try_insert(Config {
            version: 0,
            env: "dev".to_string(), // by default, a new node will be set to "dev" so we can upload its config independently of authorizations
            agents_enabled: false,
        })
        .is_err()
    {
        log::error!("Failed to insert config");
    }

    log::info!("Initialized bitcraft spacetimedb.");
    Ok(())
}
