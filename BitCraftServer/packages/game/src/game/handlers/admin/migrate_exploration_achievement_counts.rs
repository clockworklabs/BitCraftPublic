use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{exploration_chunks_state, exploration_chunks_state_v2, ExplorationChunksStateV2},
    },
};

#[spacetimedb::reducer]
pub fn migrate_exploration_achievement_counts(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut copied = 0;
    let mut skipped = 0;

    for exploration_state in ctx.db.exploration_chunks_state().iter() {
        if ctx
            .db
            .exploration_chunks_state_v2()
            .entity_id()
            .find(exploration_state.entity_id)
            .is_some()
        {
            skipped += 1;
            continue;
        }

        ctx.db.exploration_chunks_state_v2().insert(ExplorationChunksStateV2 {
            entity_id: exploration_state.entity_id,
            bitmap: exploration_state.bitmap,
            explored_chunks_count: exploration_state.explored_chunks_count,
            achievement_explored_chunks_count: exploration_state.explored_chunks_count,
        });
        copied += 1;
    }

    spacetimedb::log::info!("Copied {copied} exploration_chunks_state rows into exploration_chunks_state_v2");
    spacetimedb::log::info!("Skipped {skipped} exploration_chunks_state rows already present in exploration_chunks_state_v2");

    Ok(())
}
