use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{
        components::user_state,
        global::{
            chat_channel_permission_state, chat_channel_state, ChatChannelPermission, ChatChannelPermissionState, ChatChannelVisibility,
            MAX_CHAT_CHANNELS_PER_PLAYER, MAX_MEMBERS_PER_CHAT_CHANNELS,
        },
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn join_chat_channel(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    let chat_channel = unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    let all_player_permissions = ctx.db.chat_channel_permission_state().player_entity_id().filter(&actor_id);
    if all_player_permissions
        .filter(|p| p.rank != ChatChannelPermission::Banned as i32 && p.rank != ChatChannelPermission::PendingInvitation as i32)
        .count()
        >= MAX_CHAT_CHANNELS_PER_PLAYER
    {
        return Err("You can't join any more chat channels.".into());
    }

    let permissions = ctx
        .db
        .chat_channel_permission_state()
        .chat_channel_and_player_entity_id()
        .filter((entity_id, actor_id))
        .next();

    if let Some(mut p) = permissions {
        if p.rank == ChatChannelPermission::Banned as i32 {
            return Err("You have been banned from this chat channel.".into());
        }

        if p.rank == ChatChannelPermission::Owner as i32
            || p.rank == ChatChannelPermission::Officer as i32
            || p.rank == ChatChannelPermission::Member as i32
        {
            return Err("You're already a member of this chat channel.".into());
        }

        if p.rank == ChatChannelPermission::AccessRequested as i32 && chat_channel.visibility != ChatChannelVisibility::Public {
            return Err("You already requested access to this chat channel.".into());
        }

        if p.rank == ChatChannelPermission::PendingInvitation as i32 || p.rank == ChatChannelPermission::AccessRequested as i32 {
            p.rank = ChatChannelPermission::Member as i32;
            ctx.db.chat_channel_permission_state().entity_id().update(p);
            return Ok(());
        }

        return Err("Invalid chat channel permission state.".into());
    }

    if chat_channel.visibility == ChatChannelVisibility::Unlisted {
        return Err("This chat channel is unlisted and you can't request access to it.".into());
    }

    let all_channel_permissions = ctx.db.chat_channel_permission_state().chat_channel_entity_id().filter(entity_id);
    if all_channel_permissions
        .filter(|p| p.rank != ChatChannelPermission::Banned as i32 && p.rank != ChatChannelPermission::AccessRequested as i32)
        .count()
        >= MAX_MEMBERS_PER_CHAT_CHANNELS
    {
        return Err("This chat channel has reached members limit.".into());
    }

    let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(actor_id), "Invalid user.");
    ctx.db.chat_channel_permission_state().insert(ChatChannelPermissionState {
        entity_id: game_state::create_entity(ctx),
        chat_channel_entity_id: entity_id,
        player_entity_id: actor_id,
        identity: user.identity,
        rank: (if chat_channel.visibility == ChatChannelVisibility::Public {
            ChatChannelPermission::Member
        } else {
            ChatChannelPermission::AccessRequested
        }) as i32,
    });

    Ok(())
}
