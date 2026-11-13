use spacetimedb::ReducerContext;

use crate::messages::{
    empire_shared::{empire_state, EmpireState},
    inter_module::EmpireUpdateEmperorCrownMsg,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireUpdateEmperorCrownMsg) -> Result<(), String> {
    if ctx.db.empire_state().entity_id().find(request.empire_entity_id).is_none() {
        spacetimedb::log::warn!("EmpireUpdateEmperorCrown - Empire {} doesn't exist", request.empire_entity_id);
        return Ok(()); //This can happen when an empire is deleted
    }
    EmpireState::update_crown_status(ctx, request.empire_entity_id)
}
