use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext};

use crate::{
    game::game_state::{self},
    messages::global::{chat_channel_permission_state, ChatChannelPermission}, unwrap_or_err
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn leave_chat_channel(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let permissions = unwrap_or_err!(ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().filter((entity_id, actor_id)).next(), "You're not a member of this chat channel.");
    if permissions.rank == ChatChannelPermission::Owner as i32 {
        return Err("You can't leave a chat channel you're the owner of.".into());
    }

    if ctx.db.chat_channel_permission_state().chat_channel_and_player_entity_id().delete((entity_id, actor_id)) == 0 {
        return Err("You're not a member of this chat channel.".into());
    }

    Ok(())
}
