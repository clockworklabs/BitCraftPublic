use crate::a_i_debug_state;
use crate::messages::action_request::CheatSetDebugAiStateRequest;
use crate::messages::authentication::Role;
use crate::{game::handlers::authentication::has_role, AIDebugState};

use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
pub fn cheat_set_ai_debug_state(ctx: &ReducerContext, request: CheatSetDebugAiStateRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let debug = AIDebugState {
        entity_id: request.entity_id,
        target_entity_id: request.target_entity_id,
        target_position: request.target_position,
        current_position: request.current_position,
        current_destination: request.current_destination,
        dp: request.dp,
    };

    if ctx.db.a_i_debug_state().entity_id().find(request.entity_id).is_none() {
        ctx.db.a_i_debug_state().try_insert(debug)?;
    } else {
        ctx.db.a_i_debug_state().entity_id().update(debug);
    }

    Ok(())
}
