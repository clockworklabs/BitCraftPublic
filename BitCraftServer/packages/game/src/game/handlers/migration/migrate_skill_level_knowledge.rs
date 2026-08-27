use spacetimedb::{log, ReducerContext, Table};

use crate::{
    experience_state,
    game::handlers::authentication::has_role,
    messages::{authentication::Role, components::ExperienceState, game_util::ExperienceStack},
};

#[spacetimedb::reducer]
pub fn migrate_skill_level_knowledge(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut processed_players = 0;
    for experience_state in ctx.db.experience_state().iter() {
        for experience_stack in experience_state.experience_stacks {
            let current_level = ExperienceStack::level_for_experience(experience_stack.quantity);
            ExperienceState::grant_skill_level_knowledge(ctx, experience_state.entity_id, experience_stack.skill_id, 0, current_level);
        }

        processed_players += 1;
    }

    log::info!("Migrated skill-level knowledge for {processed_players} players");
    Ok(())
}
