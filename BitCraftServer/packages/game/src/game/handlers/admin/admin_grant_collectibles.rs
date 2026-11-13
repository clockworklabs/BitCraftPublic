use crate::{
    game::handlers::authentication::has_role, messages::authentication::Role, signed_in_player_state, unclaimed_collectibles_state,
    user_state, vault_state, UnclaimedCollectiblesState,
};
use spacetimedb::{Identity, ReducerContext, Table};
use std::str::FromStr;

#[spacetimedb::reducer]
pub fn admin_grant_collectibles(ctx: &ReducerContext, identity: String, collectibles: Vec<i32>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
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
            let mut vault = ctx.db.vault_state().entity_id().find(entity_id).unwrap();
            for collectible_id in collectibles {
                let _ = vault.add_collectible(ctx, collectible_id, true);
            }
            ctx.db.vault_state().entity_id().update(vault);
            return Ok(());
        }
    }

    // player isn't signed in or does not exist yet. We will wait until sign-in to update the vault.
    if let Some(mut unclaimed_collectibles) = ctx.db.unclaimed_collectibles_state().identity().find(identity) {
        let mut collectibles = collectibles.clone();
        unclaimed_collectibles.collectibles.append(&mut collectibles);
        ctx.db.unclaimed_collectibles_state().identity().update(unclaimed_collectibles);
    } else {
        let _ = ctx
            .db
            .unclaimed_collectibles_state()
            .try_insert(UnclaimedCollectiblesState { identity, collectibles });
    }
    Ok(())
}
