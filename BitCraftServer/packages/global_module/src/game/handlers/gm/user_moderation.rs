use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, TimeDuration};

use crate::game::game_state::create_entity;
use crate::game::handlers::authentication::has_role;
use crate::inter_module::*;
use crate::messages::action_request::UserModerationCreateUserPolicyRequest;
use crate::messages::authentication::Role;
use crate::messages::components::{UserModerationPolicy, UserModerationState};
use crate::{chat_message_state, user_moderation_state, user_state};

#[spacetimedb::reducer]
#[shared_table_reducer]
fn user_moderation_create(ctx: &ReducerContext, request: UserModerationCreateUserPolicyRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Mod) {
        return Err("Unauthorized".into());
    }

    let duration = Duration::from_millis(request.duration_ms);
    let user_moderation = UserModerationState {
        entity_id: create_entity(ctx),
        target_identity: request.target_identity,
        created_by_identity: ctx.sender(),
        user_moderation_policy: request.user_moderation_policy,
        created_time: ctx.timestamp,
        expiration_time: ctx.timestamp + TimeDuration::from(duration),
    };

    log::info!("[GM] user_moderation_create(): Adding a new instance {:?}", user_moderation);

    let delete_chat_policies = [
        UserModerationPolicy::PermanentBlockLogin,
        UserModerationPolicy::TemporaryBlockLogin,
        UserModerationPolicy::BlockChat,
        UserModerationPolicy::PermanentBlockChat,
    ];
    let delete_chat = delete_chat_policies.contains(&request.user_moderation_policy);

    log::info!("[GM] user_moderation_create(): delete_chat : {}", delete_chat);

    if delete_chat {
        // Look up the player's entity_id from their identity to delete chat messages
        if let Some(user) = ctx.db.user_state().identity().find(request.target_identity) {
            let deleted_count = ctx.db.chat_message_state().owner_entity_id().delete(user.entity_id);
            log::info!(
                "[GM] user_moderation_create(): Deleted chat messages for user identity : {}, count : {}",
                request.target_identity.to_hex(),
                deleted_count
            );
        }
    }

    let sign_out_policies = [UserModerationPolicy::PermanentBlockLogin, UserModerationPolicy::TemporaryBlockLogin];

    let sign_out_user = sign_out_policies.contains(&request.user_moderation_policy);

    if sign_out_user {
        log::info!(
            "[GM] user_moderation_create(): Trying to sign out the user ... by target_identity : {}",
            request.target_identity.to_hex()
        );

        let result = sign_player_out::send_message(ctx, request.target_identity);
        if result.is_err() {
            log::info!(
                "[GM] user_moderation_create(): Failed to sign out the user by target_identity : {}, error: {}",
                request.target_identity.to_hex(),
                result.err().unwrap()
            );
        } else {
            log::info!(
                "[GM] user_moderation_create(): Successfully signed out the user by target_identity : {}",
                request.target_identity.to_hex()
            );
        }
    }

    UserModerationState::insert_shared(ctx, user_moderation, InterModuleDestination::AllOtherRegions);

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn user_moderation_delete(ctx: &ReducerContext, policy_entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Mod) {
        return Err("Unauthorized".into());
    }

    if let Some(policy) = ctx.db.user_moderation_state().entity_id().find(policy_entity_id) {
        UserModerationState::delete_shared(ctx, policy, InterModuleDestination::AllOtherRegions);
    } else {
        return Err("Policy doesn't exist".into());
    }

    Ok(())
}

// This is implemented for debugging purposes
#[spacetimedb::reducer]
fn user_moderation_clear_all(ctx: &ReducerContext, request: UserModerationCreateUserPolicyRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Mod) {
        return Err("Unauthorized".into());
    }

    for existing_state in ctx.db.user_moderation_state().target_identity().filter(&request.target_identity) {
        log::info!(
            "[GM] user_moderation_clear_all(): filter_by_target_identity: Found existing_state {:?}",
            existing_state,
        );

        UserModerationState::delete_shared(ctx, existing_state, InterModuleDestination::AllOtherRegions);
    }

    Ok(())
}
