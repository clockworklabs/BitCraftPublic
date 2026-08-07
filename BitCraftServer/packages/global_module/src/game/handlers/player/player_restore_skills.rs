use bitcraft_macro::feature_gate;
use crate::{
    game::{game_state, handlers::authentication::has_role},
    inter_module::{send_inter_module_message, InterModuleDestination},
    messages::{
        authentication::Role,
        components::{previous_player_skills_state, user_state, NotificationSeverity, PlayerNotificationEvent},
        global::user_region_state,
        inter_module::{MessageContentsV5, RestoreSkillsMsg},
    },
    unwrap_or_err,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_restore_skills(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    if !has_role(ctx, &ctx.sender, Role::Admin) {
        if actor_id != player_entity_id {
            return Err("Unauthorized".into());
        }
    }

    let user_state = unwrap_or_err!(ctx.db.user_state().entity_id().find(player_entity_id), "Player does not exist");

    let region_id = ctx.db.user_region_state().identity().find(user_state.identity).unwrap().region_id;

    if let Some(previous_skills) = ctx.db.previous_player_skills_state().identity().find(user_state.identity) {
        let msg = RestoreSkillsMsg {
            player_entity_id,
            experience_stacks: previous_skills.experience_stacks,
        };
        send_inter_module_message(
            ctx,
            MessageContentsV5::RestoreSkills(msg),
            InterModuleDestination::Region(region_id),
        );
    } else {
        return Err("Player has no stored experience".into());
    }
    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: RestoreSkillsMsg, error: Option<String>) {
    if error.is_some() {
        // This doesn't work, message is not sent (todo)
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
