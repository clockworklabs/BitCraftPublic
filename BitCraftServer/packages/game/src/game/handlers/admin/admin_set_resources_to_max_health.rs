use std::collections::HashMap;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{resource_health_state, resource_state},
        static_data::resource_desc,
    },
};

#[spacetimedb::reducer]
pub fn admin_set_resources_to_max_health(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let resources_max_health: HashMap<i32, i32> = ctx.db.resource_desc().iter().map(|desc| (desc.id, desc.max_health)).collect();
    let mut count = 0;

    for mut health_state in ctx.db.resource_health_state().iter() {
        let resource = ctx
            .db
            .resource_state()
            .entity_id()
            .find(health_state.entity_id)
            .ok_or_else(|| format!("Missing ResourceState for resource health entity {}", health_state.entity_id))?;
        let max_health = *resources_max_health.get(&resource.resource_id).ok_or_else(|| {
            format!(
                "Missing ResourceDesc {} for resource entity {}",
                resource.resource_id, resource.entity_id
            )
        })?;

        if health_state.health != max_health {
            health_state.health = max_health;
            ctx.db.resource_health_state().entity_id().update(health_state);
            count += 1;
        }
    }

    log::info!("Updated {count} resources to maximum health");
    Ok(())
}
