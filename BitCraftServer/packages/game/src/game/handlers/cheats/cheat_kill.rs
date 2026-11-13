use crate::{game::{handlers::cheats::cheat_type::{can_run_cheat, CheatType}, reducer_helpers::health_helpers::update_health_and_check_death, terrain_chunk::TerrainChunkCache}, messages::components::health_state};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_kill(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatKill) {
        return Err("Unauthorized.".into());
    }

    let mut health_state = ctx.db.health_state().entity_id().find(entity_id).unwrap();
    if health_state.health > 0.0 {
        health_state.health = 0.0;
        update_health_and_check_death(ctx, &mut TerrainChunkCache::empty(), health_state, entity_id, None);
    }
    
    Ok(())
}
