use crate::game::coordinates::ChunkCoordinates;
use crate::game::dimensions;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::action_request::CheatDiscoverMapRequest;
use crate::messages::generic::{world_region_state, RegionExplorationInfo};
use crate::{
    exploration_chunks_state_v2, knowledge_ruins_state, location_cache, unwrap_or_err, KnowledgeLocationEntry, KnowledgeState,
    OffsetCoordinatesSmall,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
fn cheat_discover_map(ctx: &ReducerContext, request: CheatDiscoverMapRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDiscoverMap) {
        return Err("Unauthorized.".into());
    }

    let entity_id = request.target_entity_id;
    reduce(ctx, entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    let mut explored_chunks = unwrap_or_err!(
        ctx.db.exploration_chunks_state_v2().entity_id().find(entity_id),
        "Player has no exploration state"
    );
    let region = ctx.db.world_region_state().id().find(0).unwrap();

    //Explore all chunks
    let old_count = explored_chunks.explored_chunks_count;
    let mut achievement_chunks_count = 0;
    let world_width = region.world_width_chunks();
    let world_height = region.world_height_chunks();
    for x in 0..world_width {
        for y in 0..world_height {
            let chunk = ChunkCoordinates {
                x,
                z: y,
                dimension: dimensions::OVERWORLD,
            };
            if explored_chunks.explore_chunk(ctx, &chunk, Some(world_width)) {
                let region_x = x / region.region_width_chunks as i32;
                let region_z = y / region.region_height_chunks as i32;
                let region_id = (region_z * region.region_count_sqrt as i32 + region_x + 1) as u8;
                if RegionExplorationInfo::counts_toward_achievements(ctx, region_id) {
                    achievement_chunks_count += 1;
                }
            }
        }
    }
    explored_chunks.achievement_explored_chunks_count = achievement_chunks_count;

    spacetimedb::log::info!("Discovered {} chunks", explored_chunks.explored_chunks_count - old_count);
    ctx.db.exploration_chunks_state_v2().entity_id().update(explored_chunks);

    //Discover all ruins
    let ruins = ctx.db.location_cache().version().find(0).unwrap().all_ruins;
    let mut knowledge = ctx.db.knowledge_ruins_state().entity_id().find(entity_id).unwrap();
    let prev_ruin_count = knowledge.entries.len();
    knowledge.entries.clear();
    for ruin in ruins {
        let coord = OffsetCoordinatesSmall::from(ruin.coordinates);
        let knowledge_entry = KnowledgeLocationEntry {
            location: coord,
            state: KnowledgeState::Discovered,
        };
        knowledge.entries.push(knowledge_entry);
    }
    spacetimedb::log::info!("Discovered {} ruins", knowledge.entries.len() - prev_ruin_count);
    ctx.db.knowledge_ruins_state().entity_id().update(knowledge);

    Ok(())
}

#[spacetimedb::reducer]
fn cheat_undiscover_map(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDiscoverMap) {
        return Err("Unauthorized.".into());
    }

    let entity_id = player_entity_id;
    let mut explored_chunks = unwrap_or_err!(
        ctx.db.exploration_chunks_state_v2().entity_id().find(entity_id),
        "Player has no exploration state"
    );

    for val in &mut explored_chunks.bitmap {
        *val = 0;
    }
    explored_chunks.explored_chunks_count = 0;
    explored_chunks.achievement_explored_chunks_count = 0;
    ctx.db.exploration_chunks_state_v2().entity_id().update(explored_chunks);

    spacetimedb::log::info!("Player {} undiscovered all chunks", entity_id);

    Ok(())
}
