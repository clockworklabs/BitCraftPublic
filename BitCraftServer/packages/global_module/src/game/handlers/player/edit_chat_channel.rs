use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::{game_state::{self}, handlers::authentication::has_role, reducer_helpers::user_text_input_helpers::{is_user_text_input_valid, sanitize_user_inputs}},
    messages::{
        authentication::Role, global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission, ChatChannelVisibility}
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn edit_chat_channel(ctx: &ReducerContext, entity_id: u64, name: String, description: String, visibility: ChatChannelVisibility) -> Result<(), String> {
    let mut chat_channel = unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(entity_id), "Invalid chat channel");

    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        let actor_id = game_state::actor_id(ctx, true)?;

        let permissions = ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((entity_id, actor_id)).next();
        if permissions.is_none() {
            return Err("You don't have permissions to edit this chat channel.".into());
        } else if let Some(p) = permissions {
            if p.rank != ChatChannelPermission::Owner as i32 {
                return Err("You don't have permissions to edit this chat channel.".into());
            }
        }
    }

    let sanitized_name = sanitize_user_inputs(&name);

    if sanitized_name.is_empty() {
        return Err("Chat channel needs a name.".to_string());
    }

    if is_user_text_input_valid(&sanitized_name, 35, true).is_err() {
        return Err("Failed to send chat messages".into());
    }

    let lowercase_name = sanitized_name.to_lowercase();
    
    chat_channel.name = sanitized_name;
    chat_channel.lowercase_name = lowercase_name;
    chat_channel.description = description;
    chat_channel.visibility = visibility;

    ctx.db.chat_channel_state().entity_id().update(chat_channel);

    if visibility == ChatChannelVisibility::Public {
        let current_permissions = ctx.db.chat_channel_permission_state().chat_channel_entity_id().filter(entity_id);
        for mut current in current_permissions {
            if current.rank == ChatChannelPermission::AccessRequested as i32 {
                current.rank = ChatChannelPermission::Member as i32;
                
                ctx.db.chat_channel_permission_state().entity_id().update(current);
            }
        }
    }

    Ok(())
}
