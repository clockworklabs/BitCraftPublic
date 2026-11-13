use spacetimedb::{log, ReducerContext};

use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, components::player_housing_state},
    unwrap_or_return,
};

#[spacetimedb::reducer]
pub fn admin_unlock_house(ctx: &ReducerContext, player_entity_id: u64) {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        log::error!("Unauthorized.");
        return;
    }

    let mut housing_state = unwrap_or_return!(
        ctx.db.player_housing_state().entity_id().find(player_entity_id),
        "Player does not have a house"
    );

    housing_state.locked_until = ctx.timestamp;
    ctx.db.player_housing_state().entity_id().update(housing_state);
}
