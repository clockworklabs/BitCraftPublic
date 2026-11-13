use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::game_state::{self},
    messages::global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission, MAX_MEMBERS_PER_CHAT_CHANNELS},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn edit_chat_channel_member_permission(ctx: &ReducerContext, channel_entity_id: u64, player_entity_id: u64, rank: ChatChannelPermission) -> Result<(), String> {
    // This is to make it easier to not allow players to be part of more than the allowed max number of channels at once
    if rank == ChatChannelPermission::Owner || rank == ChatChannelPermission::Banned || rank == ChatChannelPermission::AccessRequested || rank == ChatChannelPermission::PendingInvitation {
        return Err("Invalid target rank.".into());
    }

    unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(channel_entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    if actor_id == player_entity_id {
        return Err("You can't edit your own permission level.".into());
    }

    let actor_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, actor_id)).next(), "You're not a member of this chat channel.");
    if actor_permissions.rank != ChatChannelPermission::Owner as i32 && actor_permissions.rank != ChatChannelPermission::Officer as i32 {
        return Err("You don't have permissions to edit other members from this chat channel.".into());
    }

    if actor_permissions.rank == rank as i32 {
        return Err("You don't have enough permissions.".into());
    }

    let mut player_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, player_entity_id)).next(), "Player is not a member of this chat channel.");

    // This is to make it easier to not allow players to be part of more than the allowed max number of channels at once
    if player_permissions.rank == ChatChannelPermission::Banned as i32 || player_permissions.rank == ChatChannelPermission::PendingInvitation as i32 {
        return Err("Player is not a member of this chat channel.".into());
    }

    if actor_permissions.rank == player_permissions.rank {
        return Err("You don't have enough permissions.".into());
    }

    if player_permissions.rank == ChatChannelPermission::AccessRequested as i32 {
        let all_channel_permissions = ctx.db.chat_channel_permission_state().player_entity_id().filter(&actor_id);
        if all_channel_permissions.filter(|p| p.rank != ChatChannelPermission::Banned as i32 && p.rank != ChatChannelPermission::AccessRequested as i32).count() >= MAX_MEMBERS_PER_CHAT_CHANNELS {
            return Err("This chat channel has reached members limit.".into());
        }
    }
    
    player_permissions.rank = rank as i32;

    ctx.db.chat_channel_permission_state().entity_id().update(player_permissions);

    Ok(())
}
