use bitcraft_macro::feature_gate;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        game_state,
        handlers::authentication::{has_role, is_authenticated},
    },
    messages::{
        action_request::PlayerSignInRequest,
        authentication::{blocked_identity, developer, Role},
        components::*,
        empire_schema::*,
        global::player_shard_state,
    },
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn sign_in(ctx: &ReducerContext, _request: PlayerSignInRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, false)?;

    let _ = ctx
        .db
        .signed_in_player_state()
        .try_insert(SignedInPlayerState { entity_id: actor_id });

    // Update last viewed empire messages
    if let Some(mut player_log) = ctx.db.empire_player_log_state().entity_id().find(&actor_id) {
        if let Some(empire_log) = ctx.db.empire_log_state().entity_id().find(&player_log.empire_entity_id) {
            player_log.last_viewed = empire_log.last_posted;
            ctx.db.empire_player_log_state().entity_id().update(player_log);
        }
    }

    // Claim pending shards
    if let Some(unclaimed_shards) = ctx.db.unclaimed_shards_state().identity().find(&ctx.sender) {
        let mut vault = ctx.db.player_shard_state().entity_id().find(&actor_id).unwrap();
        vault.shards += unclaimed_shards.shards;
        ctx.db.player_shard_state().entity_id().update(vault);
        ctx.db.unclaimed_shards_state().identity().delete(&ctx.sender);
    }

    Ok(())
}

// Called everytime a new client connects
#[spacetimedb::reducer(client_connected)]
#[feature_gate]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    // Newest connection wins. Updated on every connect so a stale connection's
    // disconnect can be told apart from the active one's.
    if let (Some(user), Some(connection_id)) = (ctx.db.user_state().identity().find(&ctx.sender), ctx.connection_id) {
        let active = ActiveConnectionState {
            entity_id: user.entity_id,
            connection_id,
        };
        if ctx.db.active_connection_state().entity_id().find(user.entity_id).is_some() {
            ctx.db.active_connection_state().entity_id().update(active);
        } else {
            ctx.db.active_connection_state().insert(active);
        }
    }

    if let Some(developer) = ctx.db.developer().identity().find(ctx.sender) {
        log::info!(
            "Developer identity connected for developer: {}, service: {}",
            developer.developer_name,
            developer.service_name
        );
        return Ok(());
    }

    if has_role(ctx, &ctx.sender, Role::SkipQueue) {
        return Ok(());
    }

    if ctx.db.blocked_identity().identity().find(ctx.sender).is_some() || !is_authenticated(ctx, &ctx.sender) {
        log::info!("Blocking identity {}", ctx.sender.to_hex());
        return Err("Unauthorized".into());
    }

    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user_state().identity().find(&ctx.sender) {
        if let Some(active) = ctx.db.active_connection_state().entity_id().find(user.entity_id) {
            // Stale: the identity already opened a newer connection, i.e. the player
            // reconnected before this disconnect was processed. Signing out now would
            // end the live session.
            if ctx.connection_id != Some(active.connection_id) {
                log::info!("Stale connection disconnected for {:?}", ctx.sender.to_hex());
                return;
            }
            ctx.db.active_connection_state().entity_id().delete(user.entity_id);
        }
        ctx.db.signed_in_player_state().entity_id().delete(user.entity_id);
    }
}
