use spacetimedb::{log, ReducerContext, Table};

use crate::{
    dimension_network_state,
    game::handlers::{authentication::has_role, server::interior_set_collapsed::interior_set_collapsed},
    messages::authentication::Role,
};

#[spacetimedb::reducer]
pub fn admin_restore_all_collapsed_ruins(ctx: &ReducerContext) {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        log::error!("Unauthorized.");
        return;
    }

    for dimension_state in ctx.db.dimension_network_state().iter() {
        if dimension_state.is_collapsed
            && interior_set_collapsed(ctx, dimension_state.entity_id, false).is_err() {
                log::error!("Failed to uncollapse ruin {}", dimension_state.entity_id);
            }
    }
}
