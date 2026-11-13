use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, global::player_shard_state},
    signed_in_player_state, unclaimed_shards_state, user_state, UnclaimedShardsState,
};
use spacetimedb::{Identity, ReducerContext, Table};
use std::str::FromStr;

#[spacetimedb::reducer]
pub fn admin_grant_shards(ctx: &ReducerContext, identity: String, amount: i32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let identity = Identity::from_str(identity.as_str());
    if identity.is_err() {
        return Err("Identity couldn't be parsed".into());
    }
    let identity = identity.unwrap();

    // If player is signed-in, directly update their vault
    if let Some(user) = ctx.db.user_state().identity().find(identity) {
        let entity_id = user.entity_id;
        if ctx.db.signed_in_player_state().entity_id().find(entity_id).is_some() {
            let mut vault = ctx.db.player_shard_state().entity_id().find(entity_id).unwrap();
            if amount >= 0 {
                vault.shards = vault.shards.saturating_add(amount as u32);
            } else {
                vault.shards = vault.shards.saturating_sub((-amount) as u32);
            }
            ctx.db.player_shard_state().entity_id().update(vault);
            return Ok(());
        }
    }

    // player isn't signed in or does not exist yet. We will wait until sign-in to update the vault.
    if let Some(mut unclaimed_shards) = ctx.db.unclaimed_shards_state().identity().find(identity) {
        if amount >= 0 {
            unclaimed_shards.shards = unclaimed_shards.shards.saturating_add(amount as u32);
        } else {
            unclaimed_shards.shards = unclaimed_shards.shards.saturating_sub((-amount) as u32);
        }
        ctx.db.unclaimed_shards_state().identity().update(unclaimed_shards);
    } else {
        let initial = if amount > 0 { amount as u32 } else { 0 };
        let _ = ctx
            .db
            .unclaimed_shards_state()
            .try_insert(UnclaimedShardsState { identity, shards: initial });
    }
    Ok(())
}
