use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::messages::action_request::{ReportPlayerChatMessage, ReportPlayerMessage};
use crate::messages::components::*;
use crate::unwrap_or_err;

const CONTEXT_SIZE: usize = 5;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn report_chat_message(ctx: &ReducerContext, request: ReportPlayerChatMessage) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let chat = unwrap_or_err!(
        ctx.db.chat_message_state().entity_id().find(request.chat_message_id),
        "Chat message not found"
    );
    let username = unwrap_or_err!(
        ctx.db.player_username_state().entity_id().find(chat.owner_entity_id),
        "Player not found"
    );

    let channel_messages: Vec<ChatMessageState> = ctx
        .db
        .chat_message_state()
        .channel_and_target_id()
        .filter((chat.channel_id, chat.target_id))
        .collect();
    let msg_index = channel_messages.iter().position(|v| v.entity_id == chat.entity_id).unwrap();
    let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
    let end_index = (msg_index + CONTEXT_SIZE).min(channel_messages.len());
    let channel_messages = channel_messages[start_index..end_index].to_vec();

    let user_messages: Vec<ChatMessageState> = ctx.db.chat_message_state().owner_entity_id().filter(chat.owner_entity_id).collect();
    let msg_index = user_messages.iter().position(|v| v.entity_id == chat.entity_id).unwrap();
    let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
    let end_index = (msg_index + CONTEXT_SIZE).min(user_messages.len());
    let user_messages = user_messages[start_index..end_index].to_vec();

    let entity_id = game_state::create_entity(ctx);
    PlayerReportState::insert_shared(
        ctx,
        PlayerReportState {
            entity_id,
            reporter_entity_id: actor_id,
            reported_player_entity_id: chat.owner_entity_id,
            reported_player_username: username.username,
            report_type: request.report_type,
            report_message: request.message,
            reported_chat_message: Some(chat),
            chat_channel_context: Some(channel_messages),
            chat_user_context: Some(user_messages),
            actioned: false,
        },
        crate::inter_module::InterModuleDestination::Global,
    );
    ctx.db.player_report_state().entity_id().delete(entity_id); //We don't actually need this report locally

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn report_player(ctx: &ReducerContext, request: ReportPlayerMessage) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let username = unwrap_or_err!(
        ctx.db.player_username_state().entity_id().find(request.player_entity_id),
        "Player not found"
    );

    let all_user_msgs: Vec<ChatMessageState> = ctx
        .db
        .chat_message_state()
        .owner_entity_id()
        .filter(request.player_entity_id)
        .collect();

    // Prepare optional outputs
    let mut reported_chat_message: Option<ChatMessageState> = None;
    let mut chat_user_context: Option<Vec<ChatMessageState>> = None;
    let mut chat_channel_context: Option<Vec<ChatMessageState>> = None;

    //If they have at least one message, pick the last as reference
    if let Some(last_msg) = all_user_msgs.last() {
        //last message is assigned the "reported message"
        reported_chat_message = Some(last_msg.clone());

        //build the user context slice around last message
        let user_index = all_user_msgs.len() - 1;
        let user_start = user_index.saturating_sub(CONTEXT_SIZE);
        let user_end = (user_index + CONTEXT_SIZE + 1).min(all_user_msgs.len());
        chat_user_context = Some(all_user_msgs[user_start..user_end].to_vec());

        //build the channel context slice around last message
        let all_channel_msgs: Vec<ChatMessageState> = ctx
            .db
            .chat_message_state()
            .channel_id()
            .filter(last_msg.channel_id)
            .collect();

        if let Some(ch_idx) = all_channel_msgs
            .iter()
            .position(|v| v.entity_id == last_msg.entity_id)
        {
            let ch_start = ch_idx.saturating_sub(CONTEXT_SIZE);
            let ch_end = (ch_idx + CONTEXT_SIZE + 1).min(all_channel_msgs.len());
            chat_channel_context = Some(all_channel_msgs[ch_start..ch_end].to_vec());
        }
    }

    let entity_id = game_state::create_entity(ctx);
    PlayerReportState::insert_shared(
        ctx,
        PlayerReportState {
            entity_id,
            reporter_entity_id: actor_id,
            reported_player_entity_id: request.player_entity_id,
            reported_player_username: username.username,
            report_type: request.report_type,
            report_message: request.message,
            reported_chat_message,
            chat_channel_context,
            chat_user_context,
            actioned: false,
        },
        crate::inter_module::InterModuleDestination::Global,
    );
    ctx.db.player_report_state().entity_id().delete(entity_id); //We don't actually need this report locally

    Ok(())
}
