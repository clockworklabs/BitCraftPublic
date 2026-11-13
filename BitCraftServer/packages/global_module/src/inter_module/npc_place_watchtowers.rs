use spacetimedb::{log, ReducerContext};

use crate::{
    game::handlers::empires::npc_empire::{get_npc_empire_id, get_or_create_npc_empire, NPC_EMPIRE_DEFAULT_NAME},
    messages::{empire_shared::*, inter_module::NpcPlaceWatchtowersMsg},
    SmallHexTile, TerrainChunkState,
};

pub fn process_message_on_destination(ctx: &ReducerContext, msg: NpcPlaceWatchtowersMsg) -> Result<(), String> {
    // Use existing NPC empire, or auto-create with defaults (0 = pick first available colors)
    let npc_empire_id = match get_npc_empire_id(ctx) {
        Ok(id) => id,
        Err(_) => get_or_create_npc_empire(ctx, NPC_EMPIRE_DEFAULT_NAME, 0, 0, 0, 0),
    };

    let mut total_chunks = 0u64;
    for wt in &msg.watchtowers {
        let coord = SmallHexTile::from(wt.location);
        let chunk_index = TerrainChunkState::chunk_index_from_coords(&coord.chunk_coordinates());

        // Upsert EmpireNodeState for this watchtower
        let node = EmpireNodeState {
            entity_id: wt.building_entity_id,
            empire_entity_id: npc_empire_id,
            chunk_index,
            energy: msg.energy,
            active: msg.energy > 0,
            upkeep: msg.upkeep,
            location: wt.location,
        };
        if ctx.db.empire_node_state().entity_id().find(wt.building_entity_id).is_some() {
            EmpireNodeState::update_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
        } else {
            EmpireNodeState::insert_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // Upsert EmpireChunkState for each chunk assigned to this watchtower
        for &chunk_idx in &wt.chunk_indexes {
            let chunk = EmpireChunkState {
                chunk_index: chunk_idx,
                empire_entity_id: npc_empire_id,
                watchtower_entity_id: wt.building_entity_id,
            };
            if ctx.db.empire_chunk_state().chunk_index().find(chunk_idx).is_some() {
                EmpireChunkState::update_shared(ctx, chunk, crate::inter_module::InterModuleDestination::AllOtherRegions);
            } else {
                EmpireChunkState::insert_shared(ctx, chunk, crate::inter_module::InterModuleDestination::AllOtherRegions);
            }
            total_chunks += 1;
        }
    }

    log::info!(
        "Placed {} NPC watchtowers with {} territory chunks for empire {} (energy={}, upkeep={})",
        msg.watchtowers.len(),
        total_chunks,
        npc_empire_id,
        msg.energy,
        msg.upkeep,
    );

    Ok(())
}
