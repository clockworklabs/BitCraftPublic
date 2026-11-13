use crate::{
    game::handlers::authentication::has_role, light_source_state, messages::authentication::Role, resource_desc, resource_state, LightSourceState,
    ResourceState,
};
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::reducer]
pub fn admin_update_resource_light_source_states(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    update_resource_light_source_states(ctx);

    Ok(())
}

pub fn update_resource_light_source_states(ctx: &ReducerContext) {
    for resource in ctx.db.resource_state().iter() {
        sync_resource_light_source_state(ctx, &resource);
    }
}

pub fn sync_resource_light_source_state(ctx: &ReducerContext, resource: &ResourceState) {
    let Some(resource_desc) = ctx.db.resource_desc().id().find(resource.resource_id) else {
        return;
    };

    if let Some(mut light) = ctx.db.light_source_state().entity_id().find(resource.entity_id) {
        let light_entity_id = light.entity_id;

        if resource_desc.light_radius == 0 {
            ctx.db.light_source_state().entity_id().delete(light_entity_id);
            log::info!("[{}] Deleting resource light source state", resource_desc.name);
        } else {
            let light_radius = resource_desc.light_radius as f32;

            if light.radius != light_radius {
                log::info!(
                    "[{}] Updating resource light source radius {} -> {}",
                    resource_desc.name,
                    light.radius,
                    light_radius
                );
                light.radius = light_radius;
                ctx.db.light_source_state().entity_id().update(light);
            }
        }
    } else if resource_desc.light_radius > 0 {
        log::info!(
            "[{}] Creating resource light source state with radius {}",
            resource_desc.name,
            resource_desc.light_radius
        );
        let _ = ctx.db.light_source_state().try_insert(LightSourceState {
            entity_id: resource.entity_id,
            radius: resource_desc.light_radius as f32,
        });
    }
}
