use spacetimedb::{log, ReducerContext, Table};

use crate::{
    enemy_state,
    game::{
        dimensions,
        handlers::{authentication::has_role, server::enemy_despawn},
    },
    messages::authentication::Role,
    mobile_entity_state,
};

#[spacetimedb::reducer]
pub fn admin_despawn_overworld_enemies(ctx: &ReducerContext) {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        log::error!("Unauthorized.");
        return;
    }

    for enemy in ctx.db.enemy_state().iter() {
        let location = ctx.db.mobile_entity_state().entity_id().find(enemy.entity_id).unwrap();
        if location.dimension == dimensions::OVERWORLD {
            enemy_despawn::reduce(ctx, enemy.entity_id, false);
        }
    }

    log::info!("Despawned all overworld enemies.");
}
