use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{player_state, TravelerTaskState},
    },
};

#[spacetimedb::reducer]
pub fn admin_migrate_traveler_tasks(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut processed_players = 0;
    for player in ctx.db.player_state().iter() {
        TravelerTaskState::generate_initial_tasks(ctx, player.entity_id);
        processed_players += 1;
    }

    log::info!("Backfilled traveler tasks for {processed_players} players");
    Ok(())
}
