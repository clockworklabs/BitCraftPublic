use std::collections::HashMap;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{handlers::authentication::has_role, world_gen::resources_log::resources_log},
    messages::{
        authentication::Role,
        generic::{resource_count, ResourceCount},
        static_data::*,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_resources_log_count(ctx: &ReducerContext, threshold: f32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let resources_log = unwrap_or_err!(ctx.db.resources_log().version().find(0), "No resource log");
    let resource_desc: HashMap<i32, ResourceDesc> = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();

    // Tally missing resources in the world
    for resource in &resources_log.resources {
        if let Some(ResourceCount {
            num_in_world: resource_count,
            ..
        }) = ctx.db.resource_count().resource_id().find(resource.resource_id)
        {
            if let Some(resource_desc) = resource_desc.get(&resource.resource_id) {
                let target_threshold = (resource.world_target as f32 * threshold) as i32;
                if !resource_desc.not_respawning && resource_count < target_threshold {
                    log::info!(
                        "Resource {} (id: {}) - [{}/{}] - {:.2}%",
                        resource_desc.name,
                        resource.resource_id,
                        resource_count,
                        resource.world_target,
                        (resource_count as f32 / resource.world_target as f32) * 100f32
                    )
                }
            }
        }
    }

    Ok(())
}
