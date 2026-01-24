use bitcraft_macro::shared_table;
use spacetimedb::{Identity, ReducerContext, Table};

use crate::game::handlers::authentication::has_role;

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[repr(i32)]
// IMPORTANT: These are sorted in order of access level, from least to most access.
pub enum Role {
    Player,
    Partner,
    SkipQueue,
    Mod,
    Gm,
    Admin,
    Relay,
}

#[spacetimedb::table(name = user_authentication_state)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct UserAuthenticationState {
    #[primary_key]
    pub identity: Identity,
    pub timestamp: Timestamp,
}

// NOTE: Ideally this would be a shared table, but because of schema migration
// limitation it isn't currently. When inserting an identity we need to manually
// do so in every module, including global.
#[spacetimedb::table(name = developer)]
#[derive(Clone, Debug)]
pub struct Developer {
    #[primary_key]
    pub identity: Identity,
    pub developer_name: String,
    pub service_name: String,
    pub email: String, // table should always remain private!
    pub is_external: bool,
}

#[spacetimedb::table(name = identity_role, public)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct IdentityRole {
    #[primary_key]
    pub identity: Identity,
    pub role: Role,
}

#[spacetimedb::table(name = server_identity)]
pub struct ServerIdentity {
    #[primary_key]
    pub version: u8,
    pub identity: Identity,
}

#[spacetimedb::table(name = blocked_identity)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct BlockedIdentity {
    #[primary_key]
    pub identity: Identity,
}

impl ServerIdentity {
    pub fn set(ctx: &ReducerContext) {
        ctx.db
            .server_identity()
            .try_insert(ServerIdentity {
                version: 0,
                identity: ctx.identity(),
            })
            .unwrap();
    }

    pub fn validate_server_or_admin(ctx: &ReducerContext) -> Result<(), String> {
        match ctx.db.server_identity().version().find(0) {
            Some(server_identity) => {
                if server_identity.identity == ctx.sender || has_role(ctx, &ctx.sender, Role::Admin) {
                    Ok(())
                } else {
                    Err("Unauthorized".into())
                }
            }
            None => Err("Server isn't initialized.".into()),
        }
    }

    pub fn validate_server_only(ctx: &ReducerContext) -> Result<(), String> {
        match ctx.db.server_identity().version().find(0) {
            Some(server_identity) => {
                if server_identity.identity == ctx.sender {
                    Ok(())
                } else {
                    Err("Unauthorized".into())
                }
            }
            None => Err("Server isn't initialized.".into()),
        }
    }
}
