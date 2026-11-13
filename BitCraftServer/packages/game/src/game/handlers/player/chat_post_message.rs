use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{self, create_entity, unix};
use crate::game::reducer_helpers::user_text_input_helpers::{is_user_text_input_valid, sanitize_user_inputs};
use crate::messages::action_request::PlayerChatPostMessageRequest;
use crate::messages::components::*;
use crate::messages::moderation_config::{region_moderation_config_state, RegionModerationConfigState};
use crate::messages::static_data::CollectibleType;
use crate::{collectible_desc, i18n, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn chat_post_message(ctx: &ReducerContext, request: PlayerChatPostMessageRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(
        ctx,
        actor_id,
        request.text,
        request.channel_id,
        request.target_id,
        request.language_code,
    )
}

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    text: String,
    channel_id: ChatChannel,
    target_id: u64,
    language_code: String,
) -> Result<(), String> {
    UserModerationState::validate_chat_privileges(ctx, &ctx.sender(), "Your chat privileges have been suspended")?;

    if text.is_empty() {
        return Err("Can't send empty chat message".to_string());
    }

    let sanitized_user_input = sanitize_user_inputs(&text);
    if is_user_text_input_valid(&sanitized_user_input, 250, false).is_err() {
        return Err("Failed to send chat messages".into());
    }

    if !matches!(
        channel_id,
        ChatChannel::Local | ChatChannel::Region | ChatChannel::LookingForGroup | ChatChannel::Trade
    ) {
        return Err("This chat channel does not accept player messages".into());
    }

    if target_id > 0 && channel_id != ChatChannel::Local {
        return Err("This regional channel shouldn't have a target".into());
    }

    i18n::validate_chat_language_code(&language_code)?;

    let player_state = unwrap_or_err!(ctx.db.player_state().entity_id().find(actor_id), "Invalid player");

    let vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(actor_id), "Player is missing some components");
    let title_id = vault
        .collectibles
        .iter()
        .filter(|c| c.activated)
        .filter_map(|c| match ctx.db.collectible_desc().id().find(c.id) {
            Some(cd) => {
                if cd.collectible_type == CollectibleType::Title {
                    Some(cd.id)
                } else {
                    None
                }
            }
            None => None,
        })
        .next();

    let username = player_state.username(ctx);
    if channel_id == ChatChannel::Region && !title_id.is_some() {
        let config = ctx.db.region_moderation_config_state().id().find(0);
        let max_messages = config
            .as_ref()
            .map_or(RegionModerationConfigState::DEFAULT_MAX_MESSAGES_PER_TIME_PERIOD, |c| {
                c.max_messages_per_time_period
            });
        let rate_limit_window = config
            .as_ref()
            .map_or(RegionModerationConfigState::DEFAULT_RATE_LIMIT_WINDOW_SEC, |c| {
                c.rate_limit_window_sec
            });
        let min_playtime = config
            .as_ref()
            .map_or(RegionModerationConfigState::DEFAULT_NEW_ACCOUNT_MIN_PLAYTIME_SEC, |c| {
                c.new_account_min_playtime_sec
            });

        if player_state.time_signed_in < min_playtime {
            let cutoff = game_state::unix(ctx.timestamp) - min_playtime;
            if player_state.sign_in_timestamp > cutoff {
                let hours = min_playtime / 3600;
                return Err(format!("Region chat is unlocked after {{0}} hours for new accounts.|~{}", hours));
            }
        }
        if username.starts_with("player") {
            return Err("You need to set your username to post in Region chat.".into());
        }

        // get all recent region messages
        let since_ts = unix(ctx.timestamp) - rate_limit_window;
        let msg_count = ctx
            .db
            .chat_message_state()
            .channel_id()
            .filter(channel_id as i32)
            .filter(|m| m.owner_entity_id == actor_id && m.timestamp >= since_ts)
            .count();
        if msg_count >= max_messages as usize {
            return Err(format!(
                "You can only send {{0}} messages per {{1}} seconds in Region chat|~{}|~{}",
                max_messages, rate_limit_window
            ));
        }
    }

    let message_entity_id = create_entity(ctx);
    if ctx
        .db
        .chat_message_state()
        .try_insert(ChatMessageState {
            entity_id: message_entity_id,
            //username: player_state.username(ctx), //I18N
            title_id: title_id.unwrap_or(0),
            text: sanitized_user_input,
            timestamp: unix(ctx.timestamp),
            owner_entity_id: actor_id,
            target_id,
            channel_id: channel_id as i32,
            //language_code: Some(language_code) //I18N
            username: i18n::dont_reformat(format!("{}/{}", language_code, player_state.username(ctx))), //I18N
        })
        .is_err()
    {
        return Err("Failed to insert chat message".into());
    }

    Ok(())
}
