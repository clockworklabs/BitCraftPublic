use spacetimedb::{ReducerContext, Table};

use crate::{
    messages::{
        empire_schema::{empire_siege_engine_state, EmpireSiegeEngineState},
        empire_shared::*,
        inter_module::*,
    },
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireStartSiegeMsg) -> Result<(), String> {
    if request.is_depleted_watchtower {
        let defending_node = unwrap_or_err!(
            ctx.db.empire_node_state().entity_id().find(request.building_entity_id),
            "This building cannot be sieged"
        );
        if defending_node.energy > 0 {
            return Err("This watchtower still has supplies".into());
        }
    } else {
        ctx.db.empire_siege_engine_state().try_insert(EmpireSiegeEngineState {
            entity_id: request.deployable_entity_id,
            building_entity_id: request.building_entity_id,
        })?;
    }

    EmpireNodeSiegeState::start_siege(
        ctx,
        request.player_entity_id,
        request.building_entity_id,
        request.supplies,
        request.building_coord.into(),
        false,
    )
}
