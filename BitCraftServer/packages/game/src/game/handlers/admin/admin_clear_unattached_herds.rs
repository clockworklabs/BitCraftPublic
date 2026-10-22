use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{attached_herds_state, building_state, placeable_state, resource_state, AttachedHerdsState},
    },
};

#[spacetimedb::reducer]
pub fn admin_clear_unattached_herds(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let mut delete_count = 0;
    for attached_herd in ctx.db.attached_herds_state().iter() {
        if ctx.db.resource_state().entity_id().find(attached_herd.entity_id).is_none() {
            if ctx.db.building_state().entity_id().find(attached_herd.entity_id).is_none() {
                if ctx.db.placeable_state().entity_id().find(attached_herd.entity_id).is_some() {
                    continue;
                }
                // No building, resource, or placeable to hold the herd.
                // This needs to be updated with extra tables if herds can be attached to more entities
                AttachedHerdsState::delete(ctx, attached_herd.entity_id);
                delete_count += 1;
            }
        }
    }

    log::info!("Deleted {delete_count} unattached herds");

    Ok(())
}
