use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, generic::globals},
    terrain_chunk_state,
};

#[spacetimedb::table(accessor = reset_chunk_index_timer, scheduled(reset_chunk_index_with_dimension, at = scheduled_at))]
pub struct ResetChunkIndexTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub dimension: u32,
}

#[spacetimedb::reducer]
pub fn reset_chunk_index_with_dimension(ctx: &ReducerContext, timer: ResetChunkIndexTimer) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }
    if let Some(mut chunk) = ctx.db.terrain_chunk_state().dimension().filter(timer.dimension).next() {
        let prev_index = chunk.chunk_index;
        chunk.chunk_index = (chunk.dimension as u64 - 1) * 1000000 + chunk.chunk_z as u64 * 1000 + chunk.chunk_x as u64 + 1;
        log::info!(
            "({}, {}, {}, {}) Setting chunk index to {}",
            chunk.chunk_x,
            chunk.chunk_z,
            chunk.dimension,
            prev_index,
            chunk.chunk_index
        );
        ctx.db.terrain_chunk_state().chunk_index().update(chunk);
    } else {
        log::error!("Couldn't find chunk for dimension {}", timer.dimension);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn reset_chunk_index(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let globals = ctx.db.globals().version().find(0).unwrap();
    log::info!("Dimension Counter: {}", globals.dimension_counter);
    for i in 2..=globals.dimension_counter {
        ctx.db
            .reset_chunk_index_timer()
            .try_insert(ResetChunkIndexTimer {
                scheduled_id: 0,
                scheduled_at: ctx.timestamp.into(),
                dimension: i,
            })
            .ok()
            .unwrap();
    }
    Ok(())
}
