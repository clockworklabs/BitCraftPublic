use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{Identity, ReducerContext};

use crate::{
    messages::{
        authentication::{
            blocked_identity, identity_role, user_authentication_state, BlockedIdentity, IdentityRole, Role, UserAuthenticationState,
        },
        generic::config,
    },
    unwrap_or_err, user_state,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn authenticate(ctx: &ReducerContext, identity: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = match identity.parse() {
        Ok(i) => i,
        Err(_) => return Err("Failed to parse identity".into()),
    };

    if let Some(mut user_authentication_state) = ctx.db.user_authentication_state().identity().find(identity) {
        user_authentication_state.timestamp = ctx.timestamp;
        UserAuthenticationState::update_shared(
            ctx,
            user_authentication_state,
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
    } else {
        UserAuthenticationState::insert_shared(
            ctx,
            UserAuthenticationState {
                identity,
                timestamp: ctx.timestamp,
            },
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
    }

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn set_role_for_identity(ctx: &ReducerContext, identity: String, role: Role) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = match identity.parse() {
        Ok(i) => i,
        Err(_) => return Err("Failed to parse identity".into()),
    };
    if let Some(mut entry) = ctx.db.identity_role().identity().find(identity) {
        entry.role = role;
        IdentityRole::update_shared(ctx, entry, crate::inter_module::InterModuleDestination::AllOtherRegions);
    } else {
        IdentityRole::insert_shared(
            ctx,
            IdentityRole { identity, role },
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
    }

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn update_role_for_player(ctx: &ReducerContext, player_entity_id: u64, role: Role) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let player = unwrap_or_err!(ctx.db.user_state().entity_id().find(player_entity_id), "Player not found");

    let identity = player.identity;

    if let Some(mut entry) = ctx.db.identity_role().identity().find(identity) {
        entry.role = role;
        IdentityRole::update_shared(ctx, entry, crate::inter_module::InterModuleDestination::AllOtherRegions);
    } else {
        IdentityRole::insert_shared(
            ctx,
            IdentityRole { identity, role },
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
    }

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn block_identity(ctx: &ReducerContext, identity: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let identity = match identity.parse() {
        Ok(i) => i,
        Err(_) => return Err("Failed to parse identity".into()),
    };

    if ctx.db.blocked_identity().identity().find(identity).is_some() {
        return Err("Identity is already blocked.".into());
    }

    BlockedIdentity::insert_shared(
        ctx,
        BlockedIdentity { identity },
        crate::inter_module::InterModuleDestination::AllOtherRegions,
    );

    Ok(())
}

pub fn is_authenticated(ctx: &ReducerContext, identity: &Identity) -> bool {
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
            return duration < Duration::from_secs(3600);
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
