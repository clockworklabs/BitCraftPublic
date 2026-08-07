use crate::game::game_state;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::inter_module::send_inter_module_message;
use crate::messages::empire_schema::empire_siege_engine_state;
use crate::messages::empire_shared::*;
use crate::unwrap_or_err;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[shared_table_reducer]
fn cheat_empire_siege_cancel(ctx: &ReducerContext, siege_node_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender, CheatType::CheatEmpireSiegeCancel) {
        return Err("Unauthorized.".into());
    }

    let passed_node = unwrap_or_err!(
        ctx.db.empire_node_siege_state().entity_id().find(&siege_node_entity_id),
        "Node doesn't exist"
    );
    for node in ctx
        .db
        .empire_node_siege_state()
        .building_entity_id()
        .filter(passed_node.building_entity_id)
    {
        EmpireNodeSiegeState::delete_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    // Destroy any siege engine related to this siege
    if let Some(siege_engine) = ctx
        .db
        .empire_siege_engine_state()
        .building_entity_id()
        .find(&passed_node.building_entity_id)
    {
        ctx.db.empire_siege_engine_state().entity_id().delete(&siege_engine.entity_id);
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::RegionDestroySiegeEngine(
                crate::messages::inter_module::RegionDestroySiegeEngineMsg {
                    deployable_entity_id: siege_engine.entity_id,
                },
            ),
            crate::inter_module::InterModuleDestination::Region(game_state::region_index_from_entity_id(siege_engine.entity_id)),
        );
    }

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn cheat_empire_siege_add_supplies(
    ctx: &ReducerContext,
    siege_node_entity_id: u64,
    supplies: i32, /* can be negative */
) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender, CheatType::CheatEmpireSiegeAddSupplies) {
        return Err("Unauthorized.".into());
    }

    let mut siege = unwrap_or_err!(
        ctx.db.empire_node_siege_state().entity_id().find(&siege_node_entity_id),
        "Siege doesn't exist"
    );

    siege.energy = siege.energy + supplies;
    if siege.energy < 0 {
        let mut node = unwrap_or_err!(
            ctx.db.empire_node_state().entity_id().find(siege.building_entity_id),
            "Building Node doesn't exist"
        );

        if siege.empire_entity_id == node.empire_entity_id {
            // reduce supplies as well
            node.energy = (node.energy + siege.energy).max(0);
            EmpireNodeState::update_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
        siege.energy = 0;
    }
    EmpireNodeSiegeState::update_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);

    Ok(())
}
