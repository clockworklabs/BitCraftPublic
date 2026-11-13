use crate::game::game_state::{self, create_entity};
use bitcraft_macro::feature_gate;
use crate::messages::components::{chat_message_state, player_report_state, player_report_state_timestamp, player_username_state, ChatMessageState, PlayerReportStateTimestamp};
use crate::messages::global::{direct_message_state, DirectMessageState};
use crate::{unwrap_or_err, PlayerReportState};
use spacetimedb::{ReducerContext, Table};

const CONTEXT_SIZE: usize = 5;

#[spacetimedb::reducer]
#[feature_gate]
pub fn report_targeted_chat_message(ctx: &ReducerContext, chat_message_id: u64, report_type: String, message: String) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;


    let chat_message = ctx.db.chat_message_state().entity_id().find(chat_message_id);

    // TODO: We can get rid of this once we fix RLS (or use Views) and use ChatMessageState for DMs
    if chat_message.is_none() {
        let direct_message = unwrap_or_err!(ctx.db.direct_message_state().entity_id().find(chat_message_id), "Message not found.");

        let username = unwrap_or_err!(ctx.db.player_username_state().entity_id().find(direct_message.sender_entity_id), "Player not found");

        let channel_messages: Vec<DirectMessageState> = ctx.db.direct_message_state().receiver_entity_id().filter(direct_message.receiver_entity_id).collect();
        let msg_index = channel_messages.iter().position(|v| v.entity_id == direct_message.entity_id).unwrap();
        let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
        let end_index = (msg_index + CONTEXT_SIZE).min(channel_messages.len());
        let channel_messages = channel_messages[start_index..end_index].to_vec();

        let user_messages: Vec<DirectMessageState> = ctx.db.direct_message_state().sender_entity_id().filter(direct_message.sender_entity_id).collect();
        let msg_index = user_messages.iter().position(|v| v.entity_id == direct_message.entity_id).unwrap();
        let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
        let end_index = (msg_index + CONTEXT_SIZE).min(user_messages.len());
        let user_messages = user_messages[start_index..end_index].to_vec();

        let entity_id = create_entity(ctx);
        let report_state = PlayerReportState {
            entity_id,
            reporter_entity_id: actor_id,
            reported_player_entity_id: direct_message.sender_entity_id,
            reported_player_username: username.username,
            report_type,
            report_message: message,
            reported_chat_message: Some(direct_message.into_chat_message_state()),
            chat_channel_context: Some(
                channel_messages
                    .into_iter()
                    .map(DirectMessageState::into_chat_message_state)
                    .collect()
            ),
            chat_user_context: Some(
                user_messages
                    .into_iter()
                    .map(DirectMessageState::into_chat_message_state)
                    .collect()
            ),
            actioned: false,
        };

        ctx.db.player_report_state().try_insert(report_state)?;
        ctx.db.player_report_state_timestamp().try_insert(PlayerReportStateTimestamp {
            entity_id,
            timestamp: game_state::unix(ctx.timestamp),
        })?;

        return Ok(());
    }

    let chat_message = chat_message.unwrap();

    let username = unwrap_or_err!(ctx.db.player_username_state().entity_id().find(chat_message.owner_entity_id), "Player not found");

    let channel_messages: Vec<ChatMessageState> = ctx.db.chat_message_state().target_id().filter(chat_message.target_id).collect();
    let msg_index = channel_messages.iter().position(|v| v.entity_id == chat_message.entity_id).unwrap();
    let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
    let end_index = (msg_index + CONTEXT_SIZE).min(channel_messages.len());
    let channel_messages = channel_messages[start_index..end_index].to_vec();

    let user_messages: Vec<ChatMessageState> = ctx.db.chat_message_state().owner_entity_id().filter(chat_message.owner_entity_id).collect();
    let msg_index = user_messages.iter().position(|v| v.entity_id == chat_message.entity_id).unwrap();
    let start_index = msg_index - CONTEXT_SIZE.min(msg_index);
    let end_index = (msg_index + CONTEXT_SIZE).min(user_messages.len());
    let user_messages = user_messages[start_index..end_index].to_vec();

    let entity_id = create_entity(ctx);
    let report_state = PlayerReportState {
        entity_id,
        reporter_entity_id: actor_id,
        reported_player_entity_id: chat_message.owner_entity_id,
        reported_player_username: username.username,
        report_type,
        report_message: message,
        reported_chat_message: Some(chat_message),
        chat_channel_context: Some(channel_messages),
        chat_user_context: Some(user_messages),
        actioned: false,
    };

    ctx.db.player_report_state().try_insert(report_state)?;
    ctx.db.player_report_state_timestamp().try_insert(PlayerReportStateTimestamp {
        entity_id,
        timestamp: game_state::unix(ctx.timestamp),
    })?;

    Ok(())
}