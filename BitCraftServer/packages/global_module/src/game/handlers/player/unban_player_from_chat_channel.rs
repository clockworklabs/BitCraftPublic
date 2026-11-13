use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::game_state::{self},
    messages::global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn unban_player_from_chat_channel(ctx: &ReducerContext, channel_entity_id: u64, player_entity_id: u64) -> Result<(), String> {
    unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(channel_entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    if actor_id == player_entity_id {
        return Err("You can't edit your own permission level.".into());
    }

    let actor_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, actor_id)).next(), "You don't have permissions to edit other members from this chat channel.s");
    if actor_permissions.rank != ChatChannelPermission::Owner as i32 && actor_permissions.rank != ChatChannelPermission::Officer as i32 {
        return Err("You don't have permissions to edit other members from this chat channel.".into());
    }

    let player_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, player_entity_id)).next(), "Player has not been banned from this chat channel.");

    if player_permissions.rank != ChatChannelPermission::Banned as i32 {
        return Err("Player has not been banned from this chat channel.".into());
    }

    if !ctx.db.chat_channel_permission_state().entity_id().delete(player_permissions.entity_id) {
        return Err("Couldn't lift the ban.".into());
    }

    Ok(())
}
