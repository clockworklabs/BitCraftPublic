use spacetimedb::{log, Identity, ReducerContext};

use crate::messages::components::{chat_message_state, user_moderation_state, user_state, UserModerationPolicy, UserModerationState};

impl UserModerationPolicy {
    // Must match the policies that delete global chat in the global module's user_moderation_create
    pub fn deletes_chat(&self) -> bool {
        matches!(
            self,
            UserModerationPolicy::PermanentBlockLogin
                | UserModerationPolicy::TemporaryBlockLogin
                | UserModerationPolicy::BlockChat
                | UserModerationPolicy::PermanentBlockChat
        )
    }
}

impl UserModerationState {
    pub fn delete_region_chat_messages(ctx: &ReducerContext, identity: &Identity) {
        let Some(user) = ctx.db.user_state().identity().find(identity) else {
            return;
        };
        let deleted_count = ctx.db.chat_message_state().owner_entity_id().delete(user.entity_id);
        log::info!(
            "Deleted {} region chat messages for moderated identity {}",
            deleted_count,
            identity.to_hex()
        );
    }

    pub fn validate_chat_privileges(ctx: &ReducerContext, identity: &Identity, error_message: &str) -> Result<(), String> {
        for existing_state in ctx.db.user_moderation_state().target_identity().filter(identity) {
            if existing_state.user_moderation_policy == UserModerationPolicy::PermanentBlockChat {
                return Err(error_message.into());
            }

            if existing_state.user_moderation_policy == UserModerationPolicy::BlockChat && ctx.timestamp < existing_state.expiration_time {
                return Err(error_message.into());
            }
        }

        Ok(())
    }
}
