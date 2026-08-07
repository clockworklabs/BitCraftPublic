use std::str::FromStr;

use spacetimedb::{Identity, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    inter_module::send_inter_module_message,
    messages::{
        authentication::*,
        components::{unclaimed_collectibles_state, unclaimed_shards_state, user_previous_region_state, user_state},
        global::{
            chat_channel_permission_state, granted_hub_item_state, premium_purchase_state, user_creation_timestamp_state, user_region_state,
        },
        inter_module::ReplaceIdentityMsg,
    },
};

#[spacetimedb::reducer]
pub fn admin_replace_identity(ctx: &ReducerContext, old_identity: String, new_identity: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }
    let old_identity = match Identity::from_str(&old_identity) {
        Ok(i) => i,
        Err(_) => return Err("Couldn't parse old identity".into()),
    };
    let new_identity = match Identity::from_str(&new_identity) {
        Ok(i) => i,
        Err(_) => return Err("Couldn't parse new identity".into()),
    };

    spacetimedb::log::info!("Replacing identity {old_identity} -> {new_identity}");

    if let Some(mut v) = ctx.db.user_authentication_state().identity().find(old_identity) {
        ctx.db.user_authentication_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.user_authentication_state().insert(v);
    }
    if let Some(mut v) = ctx.db.developer().identity().find(old_identity) {
        ctx.db.developer().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.developer().insert(v);
    }
    if let Some(mut v) = ctx.db.identity_role().identity().find(old_identity) {
        ctx.db.identity_role().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.identity_role().insert(v);
    }
    if let Some(mut v) = ctx.db.blocked_identity().identity().find(old_identity) {
        ctx.db.blocked_identity().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.blocked_identity().insert(v);
    }
    if let Some(mut v) = ctx.db.user_state().identity().find(old_identity) {
        ctx.db.user_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.user_state().insert(v);
    }
    if let Some(mut v) = ctx.db.unclaimed_shards_state().identity().find(old_identity) {
        ctx.db.unclaimed_shards_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.unclaimed_shards_state().insert(v);
    }
    if let Some(mut v) = ctx.db.unclaimed_collectibles_state().identity().find(old_identity) {
        ctx.db.unclaimed_collectibles_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.unclaimed_collectibles_state().insert(v);
    }
    if let Some(mut v) = ctx.db.user_previous_region_state().identity().find(old_identity) {
        ctx.db.user_previous_region_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.user_previous_region_state().insert(v);
    }

    if let Some(mut v) = ctx.db.user_region_state().identity().find(old_identity) {
        ctx.db.user_region_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.user_region_state().insert(v);
    }
    if let Some(mut v) = ctx.db.granted_hub_item_state().iter().filter(|v| v.identity == old_identity).next() {
        ctx.db.granted_hub_item_state().entity_id().delete(v.entity_id);
        v.identity = new_identity;
        ctx.db.granted_hub_item_state().insert(v);
    }
    for mut v in ctx.db.chat_channel_permission_state().identity().filter(old_identity) {
        ctx.db.chat_channel_permission_state().entity_id().delete(v.entity_id);
        v.identity = new_identity;
        ctx.db.chat_channel_permission_state().insert(v);
    }
    if let Some(mut v) = ctx.db.user_creation_timestamp_state().identity().find(old_identity) {
        ctx.db.user_creation_timestamp_state().identity().delete(old_identity);
        v.identity = new_identity;
        ctx.db.user_creation_timestamp_state().insert(v);
    }
    if let Some(mut v) = ctx.db.premium_purchase_state().iter().filter(|v| v.identity == old_identity).next() {
        ctx.db.premium_purchase_state().entity_id().delete(v.entity_id);
        v.identity = new_identity;
        ctx.db.premium_purchase_state().insert(v);
    }

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::ReplaceIdentity(ReplaceIdentityMsg {
            old_identity,
            new_identity,
        }),
        crate::inter_module::InterModuleDestination::AllOtherRegions,
    );

    Ok(())
}
