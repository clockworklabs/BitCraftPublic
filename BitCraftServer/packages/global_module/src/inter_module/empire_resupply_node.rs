use spacetimedb::ReducerContext;

use crate::{
    game::handlers::empires::empires_shared::empire_resupply_node_validate,
    messages::{empire_shared::*, inter_module::*},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireResupplyNodeMsg) -> Result<(), String> {
    empire_resupply_node_validate(ctx, request.player_entity_id, request.building_entity_id)?;

    // Find the building empire affiliation
    let mut empire_node = unwrap_or_err!(
        ctx.db.empire_node_state().entity_id().find(request.building_entity_id),
        "The building needs to be part of an empire to receive a hexite capsule"
    );
    let empire_entity_id = empire_node.empire_entity_id;

    let recalculate_upkeep = empire_node.energy == 0;
    let previous_supplies = empire_node.energy;
    empire_node.add_energy(ctx, request.supplies_count, None);

    if recalculate_upkeep {
        // Try to activate the node, but it's OK if you can't.
        let _ = empire_node.activate(ctx, 0);
    }

    if previous_supplies != empire_node.energy {
        EmpireNodeState::update_shared(ctx, empire_node, super::InterModuleDestination::AllOtherRegions);
    }

    if recalculate_upkeep {
        EmpireState::update_empire_upkeep(ctx, empire_entity_id);
    }

    Ok(())
}
