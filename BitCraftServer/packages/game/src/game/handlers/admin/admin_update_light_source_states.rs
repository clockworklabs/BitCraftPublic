use crate::{
    building_desc, building_state, game::handlers::authentication::has_role, light_source_state, messages::authentication::Role,
    unwrap_or_err, LightSourceState,
};
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::reducer]
pub fn admin_update_light_source_states(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    for building in ctx.db.building_state().iter() {
        let building_desc = unwrap_or_err!(
            ctx.db.building_desc().id().find(&building.building_description_id),
            "Unknown BuildingDesc with id {}",
            building.building_description_id
        );

        // If already exist, update or delete.
        if let Some(mut light) = ctx.db.light_source_state().entity_id().find(&building.entity_id) {
            let light_entity_id = light.entity_id;

            // Delete light
            if building_desc.light_radius == 0 {
                ctx.db.light_source_state().entity_id().delete(&light_entity_id);
                log::info!("[{}] Deleting light source state", building_desc.name);
            } else {
                let light_radius = building_desc.light_radius as f32;

                if light.radius != light_radius {
                    log::info!(
                        "[{}] Updating building light source radius {} -> {}",
                        building_desc.name,
                        light.radius,
                        light_radius
                    );
                    light.radius = building_desc.light_radius as f32;
                    ctx.db.light_source_state().entity_id().update(light);
                }
            }
        } else {
            // If not exist, check if it needs to be created.
            if building_desc.light_radius > 0 {
                log::info!(
                    "[{}] Creating light source state with radius {}",
                    building_desc.name,
                    building_desc.light_radius
                );
                let _ = ctx.db.light_source_state().try_insert(LightSourceState {
                    entity_id: building.entity_id,
                    radius: building_desc.light_radius as f32,
                });
            }
        }
    }

    Ok(())
}
