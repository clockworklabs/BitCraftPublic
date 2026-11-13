use spacetimedb::ReducerContext;

use crate::messages::{components::player_state, empire_shared::EmpireState, inter_module::EmpireRemoveCrownMsg};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireRemoveCrownMsg) -> Result<(), String> {
    if ctx.db.player_state().entity_id().find(request.player_entity_id).is_none() {
        return Err("Empire doesn't exist".into());
    }
    EmpireState::remove_crown_status(ctx, request.player_entity_id);
    Ok(())
}
