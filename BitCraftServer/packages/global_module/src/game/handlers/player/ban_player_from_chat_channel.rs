use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{components::user_state, global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission, ChatChannelPermissionState}},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn ban_player_from_chat_channel(ctx: &ReducerContext, channel_entity_id: u64, player_entity_id: u64) -> Result<(), String> {
    unwrap_or_err!(ctx.db.chat_channel_state().entity_id().find(channel_entity_id), "Invalid chat channel");

    let actor_id = game_state::actor_id(ctx, true)?;

    if actor_id == player_entity_id {
        return Err("You can't edit your own permission level.".into());
    }

    let actor_permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, actor_id)).next(), "You don't have permissions to edit other members from this chat channel.s");
    if actor_permissions.rank != ChatChannelPermission::Owner as i32 && actor_permissions.rank != ChatChannelPermission::Officer as i32 {
        return Err("You don't have permissions to ban other members from this chat channel.".into());
    }

    let player_permissions = ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((channel_entity_id, player_entity_id)).next();
    if let Some(mut player_permissions) = player_permissions {
        player_permissions.rank = ChatChannelPermission::Banned as i32;
        ctx.db.chat_channel_permission_state().entity_id().update(player_permissions);
    } else {
        let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(player_entity_id), "Invalid user.");
        ctx.db.chat_channel_permission_state().insert(ChatChannelPermissionState {
            entity_id: game_state::create_entity(ctx),
            chat_channel_entity_id: channel_entity_id,
            player_entity_id,
            identity: user.identity,
            rank: ChatChannelPermission::Banned as i32,
        });
    }

    Ok(())
}
