use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{self, create_entity, unix};
use crate::game::handlers::authentication::has_role;
use crate::game::reducer_helpers::user_text_input_helpers::{is_user_text_input_valid, sanitize_user_inputs};
use crate::messages::action_request::PlayerChatPostMessageRequest;
use crate::messages::authentication::{identity_role, Role};
use crate::messages::components::*;
use crate::messages::global::{
    chat_channel_permission_state, chat_channel_state, direct_message_state, ChatChannelPermission, DirectMessageState,
};
use crate::{i18n, unwrap_or_err};

#[spacetimedb::reducer]
pub fn chat_post_targeted_message(ctx: &ReducerContext, request: PlayerChatPostMessageRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    // PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp); // TODO: Maybe we want to do this in the future, but not trivial. It would require sending intermodule messages.
    reduce(ctx, actor_id, request.text, request.target_id, request.language_code)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, text: String, target_entity_id: u64, language_code: String) -> Result<(), String> {
    if text.len() <= 0 {
        return Err(format!("Can't send empty chat message"));
    }

    if let Err(_) = is_user_text_input_valid(&text, 250, false) {
        return Err("Failed to send chat messages".into());
    }

    UserModerationState::validate_chat_privileges(ctx, actor_id, "Your chat priveleges have been suspended")?;

    let username = unwrap_or_err!(ctx.db.player_username_state().entity_id().find(actor_id), "Invalid player").username;

    let timestamp = unix(ctx.timestamp);
    let sanitized_user_input = sanitize_user_inputs(&text);

    //TODO: Add a mapping from Role to CollectibleId somewhere
    let title_id = match ctx.db.identity_role().identity().find(ctx.sender) {
        Some(identity_role) => match identity_role.role {
            Role::Mod => 3,
            Role::Gm => 2,
            Role::Admin => 1,
            _ => 0,
        },
        None => 0,
    };

    let message_entity_id = create_entity(ctx);

    // TODO: We can get rid of this once we fix RLS (or use Views) and use ChatMessageState for DMs
    if ctx.db.chat_channel_state().entity_id().find(target_entity_id).is_none() {
        if ctx.db.player_username_state().entity_id().find(target_entity_id).is_none() {
            return Err("Invalid message target".into());
        } else if ctx.db.signed_in_player_state().entity_id().find(target_entity_id).is_none() {
            return Err("Player is not online".into());
        }

        if ctx
            .db
            .direct_message_state()
            .try_insert(DirectMessageState {
                entity_id: message_entity_id,
                username,
                title_id,
                text: sanitized_user_input,
                timestamp,
                sender_entity_id: actor_id,
                receiver_entity_id: target_entity_id,
                language_code: Some(language_code),
            })
            .is_err()
        {
            return Err("Failed to insert chat message".into());
        }

        return Ok(());
    }

    if !has_role(ctx, &ctx.sender, Role::Gm) {
        let permissions = ctx
            .db
            .chat_channel_permission_state()
            .chat_channel_and_player_entity_id()
            .filter((target_entity_id, actor_id))
            .next();
        if permissions.is_none() {
            return Err("You don't have permissions to send messages to this chat channel.".into());
        } else if let Some(p) = permissions {
            if p.rank != ChatChannelPermission::Member as i32
                && p.rank != ChatChannelPermission::Officer as i32
                && p.rank != ChatChannelPermission::Owner as i32
            {
                return Err("You don't have permissions to send messages to this chat channel.".into());
            }
        }
    }

    if ctx
        .db
        .chat_message_state()
        .try_insert(ChatMessageState {
            entity_id: message_entity_id,
            //username, //I18N
            title_id,
            text: sanitized_user_input,
            timestamp,
            owner_entity_id: actor_id,
            target_id: target_entity_id,
            channel_id: 0,
            //language_code: Some(language_code) //I18N
            username: i18n::dont_reformat(format!("{language_code}/{username}")), //I18N
        })
        .is_err()
    {
        return Err("Failed to insert chat message".into());
    }

    Ok(())
}
