use std::time::Duration;

use spacetimedb::{Identity, ReducerContext};

use crate::messages::{
    authentication::{identity_role, user_authentication_state, Role},
    generic::config,
};

pub fn is_authenticated(ctx: &ReducerContext, identity: &Identity) -> bool {
    const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;

    match ctx.db.config().version().find(0) {
        Some(config) => {
            if config.env == "dev" {
                return true; // dev config allows all operations
            }
        }
        None => return true, // no config yet allows all operation
    }

    if let Some(entry) = ctx.db.user_authentication_state().identity().find(identity) {
        if let Some(duration) = ctx.timestamp.duration_since(entry.timestamp) {
            return duration < Duration::from_secs(SECONDS_IN_A_DAY);
        }
    }
    false
}

pub fn has_role(ctx: &ReducerContext, identity: &Identity, role: Role) -> bool {
    match ctx.db.config().version().find(0) {
        Some(config) => {
            if config.env == "dev" {
                return true; // dev config allows all operations
            }
        }
        None => return true, // no config yet allows all operation
    }
    matches!(ctx.db.identity_role().identity().find(identity), Some(entry) if entry.role as i32 >= role as i32)
}

pub fn has_role_no_dev(ctx: &ReducerContext, identity: &Identity, role: Role) -> bool {
    matches!(ctx.db.identity_role().identity().find(identity), Some(entry) if entry.role as i32 >= role as i32)
}
