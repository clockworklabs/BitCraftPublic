use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{game_state, handlers::authentication::has_role},
    inter_module::send_inter_module_message,
    messages::{
        authentication::Role,
        empire_schema::empire_siege_engine_state,
        empire_shared::{empire_node_siege_state, EmpireNodeSiegeState},
        inter_module::RegionDestroySiegeEngineMsg,
    },
};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_clean_sieges(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut parsed = 0;
    let mut unmarked = 0;
    let mut terminated = 0;

    // Check all active sieges - remove matching inactive sieges (which are marks for siege) or itself if it's the only instance of active siege
    for active_siege in ctx.db.empire_node_siege_state().iter().filter(|node| node.active) {
        parsed += 1;
        for inactive_siege in ctx
            .db
            .empire_node_siege_state()
            .building_entity_id()
            .filter(active_siege.building_entity_id)
            .filter(|node| !node.active)
        {
            unmarked += 1;
            EmpireNodeSiegeState::delete_shared(ctx, inactive_siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
        let active_sieges_for_building = ctx
            .db
            .empire_node_siege_state()
            .building_entity_id()
            .filter(active_siege.building_entity_id)
            .filter(|node| node.active)
            .count();
        if active_sieges_for_building != 2 {
            // Destroy any siege engine related to this siege
            if let Some(siege_engine) = ctx
                .db
                .empire_siege_engine_state()
                .building_entity_id()
                .find(active_siege.building_entity_id)
            {
                ctx.db.empire_siege_engine_state().entity_id().delete(siege_engine.entity_id);
                send_inter_module_message(
                    ctx,
                    crate::messages::inter_module::MessageContentsV5::RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg {
                        deployable_entity_id: siege_engine.entity_id,
                    }),
                    crate::inter_module::InterModuleDestination::Region(game_state::region_index_from_entity_id(
                        active_siege.building_entity_id,
                    )),
                );
            }
            // delete ALL active sieges
            for active_siege_for_building in ctx
                .db
                .empire_node_siege_state()
                .building_entity_id()
                .filter(active_siege.building_entity_id)
            {
                EmpireNodeSiegeState::delete_shared(
                    ctx,
                    active_siege_for_building,
                    crate::inter_module::InterModuleDestination::AllOtherRegions,
                );
            }

            terminated += active_sieges_for_building;
        }
    }

    log::info!("Parsed {parsed} sieges");
    if unmarked > 0 {
        log::info!("Unmarked {unmarked} conflicting sieges");
    }
    if terminated > 0 {
        log::info!("Terminated {terminated} broken sieges");
    }
    Ok(())
}
