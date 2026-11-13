use spacetimedb::ReducerContext;

use crate::{
    messages::{empire_shared::empire_state, inter_module::DeleteEmpireMsg},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: DeleteEmpireMsg) -> Result<(), String> {
    let empire = unwrap_or_err!(
        ctx.db.empire_state().entity_id().find(request.empire_entity_id),
        "Empire doesn't exist"
    );
    empire.delete(ctx);

    Ok(())
}
