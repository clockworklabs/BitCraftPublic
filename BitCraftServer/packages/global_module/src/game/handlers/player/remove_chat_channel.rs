use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::{game_state::{self}, handlers::authentication::has_role},
    messages::{
        authentication::Role, global::{chat_channel_permission_state, chat_channel_state, ChatChannelPermission}
    }, unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn remove_chat_channel(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        let actor_id = game_state::actor_id(ctx, true)?;

        let permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((entity_id, actor_id)).next(), "You don't have permissions to remove this chat channel.");
        if permissions.rank != ChatChannelPermission::Owner as i32 {
            return Err("You don't have permissions to remove this chat channel.".into());
        }
    }

    if !ctx.db.chat_channel_state().entity_id().delete(entity_id) {
        return Err("Invalid chat channel".into());
    }

    let all_channel_permissions = ctx.db.chat_channel_permission_state().chat_channel_entity_id().filter(entity_id);
    for channel_permission in all_channel_permissions {
        ctx.db.chat_channel_permission_state().entity_id().delete(channel_permission.entity_id);
    }

    Ok(())
}
