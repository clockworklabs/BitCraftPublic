use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{create_entity, unix};
use crate::messages::components::{moderation_action_log_entry, ModerationActionLogEntry};
use crate::{game::handlers::authentication::has_role, messages::authentication::Role};

#[spacetimedb::reducer]
#[expect(clippy::too_many_arguments, reason = "reducer arguments are part of the client API")]
pub fn admin_log_moderation_action(
    ctx: &ReducerContext,
    report_entity_id: u64,
    reported_player_entity_id: u64,
    admin_name: String,
    reported_player_username: String,
    action_type: String,
    moderation_notice: String,
    details: String,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let entity_id = create_entity(ctx);
    if ctx
        .db
        .moderation_action_log_entry()
        .try_insert(ModerationActionLogEntry {
            entity_id,
            report_entity_id,
            reported_player_entity_id,
            admin_name,
            reported_player_username,
            action_type,
            moderation_notice,
            details,
            timestamp: unix(ctx.timestamp),
        })
        .is_err()
    {
        return Err("Failed to insert moderation action log entry".into());
    }

    Ok(())
}
