use spacetimedb::ReducerContext;

use crate::messages::{components::*, inter_module::RegionDestroySiegeEngineMsg};

pub fn process_message_on_destination(ctx: &ReducerContext, request: RegionDestroySiegeEngineMsg) -> Result<(), String> {
    ctx.db.mobile_entity_state().entity_id().delete(request.deployable_entity_id);
    ctx.db.deployable_state_v2().entity_id().delete(request.deployable_entity_id);

    Ok(())
}
