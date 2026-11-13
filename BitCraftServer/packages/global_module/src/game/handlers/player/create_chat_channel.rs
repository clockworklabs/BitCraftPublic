use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state::{self}, reducer_helpers::user_text_input_helpers::{is_user_text_input_valid, sanitize_user_inputs}},
    messages::{
        components::{user_state, UserModerationState}, global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission, ChatChannelPermissionState, ChatChannelState, ChatChannelVisibility, MAX_CHAT_CHANNELS_PER_PLAYER}
    }, unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn create_chat_channel(ctx: &ReducerContext, name: String, description: String, visibility: ChatChannelVisibility) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    
    UserModerationState::validate_chat_privileges(ctx, &ctx.sender(), "Your chat privileges have been suspended")?;

    let mut permissions = ctx.db.chat_channel_permission_state().player_entity_id().filter(&actor_id);
    if permissions.any(|p| p.rank == (ChatChannelPermission::Owner as i32)) { // This assumes all permission states are chat channel related, which is true atm in the global module
        return Err("You can only own 1 chat channel.".into());
    }

    if permissions.filter(|p| p.rank != ChatChannelPermission::Banned as i32 && p.rank != ChatChannelPermission::PendingInvitation as i32).count() >= MAX_CHAT_CHANNELS_PER_PLAYER {
        return Err("You can't join any more chat channels.".into());
    }
    
    let sanitized_name = sanitize_user_inputs(&name);

    validate_chat_channel_name(&sanitized_name)?;

    let lowercase_name = sanitized_name.to_lowercase();

    let entity_id = game_state::create_entity(ctx);

    ctx.db.chat_channel_state().insert(ChatChannelState {
        entity_id,
        name: sanitized_name,
        lowercase_name,
        description,
        visibility
    });

    let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(actor_id), "Invalid user.");
    ctx.db.chat_channel_permission_state().insert(ChatChannelPermissionState {
        entity_id: game_state::create_entity(ctx),
        chat_channel_entity_id: entity_id,
        player_entity_id: actor_id,
        identity: user.identity,
        rank: ChatChannelPermission::Owner as i32,
    });

    Ok(())
}

const MIN_LENGTH: usize = 2;
const MAX_LENGTH: usize = 35;

pub fn validate_chat_channel_name(name: &str) -> Result<(), String> {
    if name.len() < MIN_LENGTH {
        return Err("This name is too short.".into());
    }

    if name.len() > MAX_LENGTH {
        return Err("This name is too long.".into());
    }

    if is_user_text_input_valid(name, MAX_LENGTH, true).is_err() {
        return Err("Invalid name".into());
    }

    Ok(())
}