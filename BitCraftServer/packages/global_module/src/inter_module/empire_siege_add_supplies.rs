use spacetimedb::ReducerContext;

use crate::{
    messages::{empire_shared::*, inter_module::*},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireSiegeAddSuppliesMsg) -> Result<(), String> {
    let mut siege = unwrap_or_err!(
        ctx.db.empire_node_siege_state().entity_id().find(request.siege_entity_id),
        "Siege doesn't exist"
    );
    siege.energy += request.supplies;
    EmpireNodeSiegeState::update_shared(ctx, siege, super::InterModuleDestination::AllOtherRegions);

    Ok(())
}
