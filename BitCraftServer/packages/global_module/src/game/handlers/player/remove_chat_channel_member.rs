use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::game_state::{self},
    messages::global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn remove_chat_channel_member(ctx: &ReducerContext, channel_entity_id: u64, player_entity_id: u64) -> Result<(), String> {
    unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(channel_entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    let actor_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, actor_id)).next(), "You don't have permissions to remove members from this chat channel.");
    if player_entity_id != actor_id && actor_permissions.rank != ChatChannelPermission::Owner as i32 && actor_permissions.rank != ChatChannelPermission::Officer as i32 {
        return Err("You don't have permissions to remove other members from this chat channel.".into());
    }

    if player_entity_id != actor_id {
        let player_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, player_entity_id)).next(), "Player is not a member of this chat channel.");
        if player_permissions.rank == ChatChannelPermission::Owner as i32 {
            return Err("You can't remove the chat channel owner.".into());
        } else if player_permissions.rank == ChatChannelPermission::Officer as i32 && actor_permissions.rank == ChatChannelPermission::Officer as i32 {
            return Err("You can't remove another chat channel officer.".into());
        }
    } else if actor_permissions.rank == ChatChannelPermission::Owner as i32 {
        return Err("As the owner, you can't leave this chat channel.".into());
    }

    if ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().delete((channel_entity_id, player_entity_id)) == 0 {
        return Err("Member removal failed.".into());
    }

    Ok(())
}
