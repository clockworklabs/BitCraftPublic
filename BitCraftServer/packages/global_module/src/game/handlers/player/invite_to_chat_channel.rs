use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{components::user_state, global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission, ChatChannelPermissionState, MAX_MEMBERS_PER_CHAT_CHANNELS}},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn invite_to_chat_channel(ctx: &ReducerContext, channel_entity_id: u64, player_entity_id: u64) -> Result<(), String> {
    unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(channel_entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    let actor_permission = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, actor_id)).next(), "You don't have permissions to invite players to join this chat channel.");
    if actor_permission.rank != ChatChannelPermission::Owner as i32 && actor_permission.rank != ChatChannelPermission::Officer as i32 {
        return Err("You don't have permission to invite players to join this chat channel.".into());
    }

    let all_channel_permissions = ctx.db.chat_channel_permission_state().chat_channel_entity_id().filter(channel_entity_id);
    if all_channel_permissions.filter(|p| p.rank != ChatChannelPermission::Banned as i32 && p.rank != ChatChannelPermission::AccessRequested as i32).count() >= MAX_MEMBERS_PER_CHAT_CHANNELS {
        return Err("This chat channel has reached members limit.".into());
    }

    let current_permissions = ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, player_entity_id)).next();
    if let Some(mut p) = current_permissions {
        if p.rank == ChatChannelPermission::Banned as i32 {
            return Err("This player has been banned from this chat channel.".into());
        }
        if p.rank == ChatChannelPermission::PendingInvitation as i32 {
            return Err("This player has already been invited to this chat channel.".into());
        }
        if p.rank == ChatChannelPermission::AccessRequested as i32 {
            p.rank = ChatChannelPermission::Member as i32;

            ctx.db.chat_channel_permission_state().entity_id().update(p);

            return Ok(());
        }

        return Err("This player is already a member of this chat channel.".into());
    }

    let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(player_entity_id), "Invalid user.");
    ctx.db.chat_channel_permission_state().insert(ChatChannelPermissionState {
        entity_id: game_state::create_entity(ctx),
        chat_channel_entity_id: channel_entity_id,
        player_entity_id,
        identity: user.identity,
        rank: ChatChannelPermission::PendingInvitation as i32,
    });

    Ok(())
}
