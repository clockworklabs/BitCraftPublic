use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{self, create_entity, unix};
use crate::game::reducer_helpers::user_text_input_helpers::{is_user_text_input_valid, sanitize_user_inputs};
use crate::messages::action_request::PlayerChatPostMessageRequest;
use crate::messages::components::*;
use crate::messages::static_data::CollectibleType;
use crate::{collectible_desc, i18n, unwrap_or_err};

#[spacetimedb::reducer]
pub fn chat_post_message(ctx: &ReducerContext, request: PlayerChatPostMessageRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
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

const MAX_MESSAGES_PER_TIME_PERIOD: usize = 3;
const RATE_LIMIT_WINDOW_SEC: i32 = 15;
const TWO_HOURS: i32 = 60 * 60 * 2;

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    text: String,
    channel_id: ChatChannel,
    target_id: u64,
    language_code: String,
) -> Result<(), String> {
    if text.len() <= 0 {
        return Err(format!("Can't send empty chat message"));
    }

    if let Err(_) = is_user_text_input_valid(&text, 250, false) {
        return Err("Failed to send chat messages".into());
    }

    let sanitized_user_input = sanitize_user_inputs(&text);

    let player_state = unwrap_or_err!(ctx.db.player_state().entity_id().find(&actor_id), "Invalid player");

    UserModerationState::validate_chat_privileges(ctx, actor_id, "Your chat priveleges have been suspended")?;

    if target_id > 0 && channel_id != ChatChannel::Local {
        return Err("This regional channel shouldn't have a target".into());
    }

    let vault = unwrap_or_err!(
        ctx.db.vault_state().entity_id().find(&actor_id),
        "Player is missing some components"
    );
    let title_id = vault
        .collectibles
        .iter()
        .filter(|c| c.activated)
        .filter_map(|c| match ctx.db.collectible_desc().id().find(&c.id) {
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
        if player_state.time_signed_in < TWO_HOURS {
            let two_hours_ago = game_state::unix(ctx.timestamp) - TWO_HOURS;
            if player_state.sign_in_timestamp > two_hours_ago {
                return Err("Region chat is unlocked after two hours for new accounts.".into());
            }
        }
        if username.starts_with("player") {
            return Err("You need to set your username to post in Region chat.".into());
        }

        // get all recent region messages
        let since_ts = unix(ctx.timestamp) - RATE_LIMIT_WINDOW_SEC;
        let msg_count = ctx
            .db
            .chat_message_state()
            .channel_id()
            .filter(channel_id as i32)
            .filter(|m| m.owner_entity_id == actor_id && m.timestamp >= since_ts)
            .count();
        if msg_count >= MAX_MESSAGES_PER_TIME_PERIOD {
            return Err(format!(
                "You can only send {{0}} messages per {{1}} seconds in Region chat|~{}|~{}",
                MAX_MESSAGES_PER_TIME_PERIOD, RATE_LIMIT_WINDOW_SEC
            )
            .into());
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
            target_id: target_id,
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
