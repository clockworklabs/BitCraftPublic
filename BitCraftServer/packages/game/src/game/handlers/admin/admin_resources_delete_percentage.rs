use spacetimedb::{log, rand::seq::SliceRandom, ReducerContext};

use crate::{
    agents::crumb_trail_clean_up_agent,
    game::{
        autogen::_delete_entity::delete_entity, handlers::authentication::has_role, reducer_helpers::footprint_helpers::delete_footprint,
        world_gen::resources_log::resources_log,
    },
    messages::{
        authentication::Role,
        components::{light_source_state, resource_state},
        generic::resource_count,
    },
    unwrap_or_err, AttachedHerdsState,
};

#[spacetimedb::reducer]
pub fn admin_resources_delete_percentage(
    ctx: &ReducerContext,
    resource_id: i32,
    percentage: f32,
    update_resources_log: bool,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    if !(0.0..=100.0).contains(&percentage) {
        return Err("Percentage must be between 0.0 and 100.0".into());
    }

    if percentage == 0.0 {
        return Ok(());
    }

    let mut resource_entity_ids: Vec<u64> = ctx
        .db
        .resource_state()
        .resource_id()
        .filter(resource_id)
        .map(|resource| resource.entity_id)
        .collect();

    let total_matching = resource_entity_ids.len();
    if total_matching == 0 {
        log::info!(
            "admin_resources_delete_percentage resource_id={} total_matching=0 percentage={:.2} deleted=0",
            resource_id,
            percentage
        );
        return Ok(());
    }

    let delete_count = (((total_matching as f32) * (percentage / 100.0)).round() as usize).min(total_matching);
    if delete_count == 0 {
        log::info!(
            "admin_resources_delete_percentage resource_id={} total_matching={} percentage={:.2} deleted=0",
            resource_id,
            total_matching,
            percentage
        );
        return Ok(());
    }

    resource_entity_ids.shuffle(&mut ctx.rng());

    for entity_id in resource_entity_ids.into_iter().take(delete_count) {
        permanently_delete_resource(ctx, entity_id, resource_id);
    }

    if update_resources_log {
        let mut resources_log = unwrap_or_err!(ctx.db.resources_log().version().find(0), "Failed to get resources log");

        for resource_info in resources_log.resources.iter_mut() {
            if resource_info.resource_id == resource_id {
                resource_info.world_target -= delete_count as i32;
                ctx.db.resources_log().version().update(resources_log);
                break;
            }
        }
    }

    log::info!(
        "admin_resources_delete_percentage resource_id={} total_matching={} percentage={:.2} deleted={}",
        resource_id,
        total_matching,
        percentage,
        delete_count
    );

    Ok(())
}

fn permanently_delete_resource(ctx: &ReducerContext, entity_id: u64, resource_id: i32) {
    if ctx.db.resource_state().entity_id().delete(&entity_id) {
        crumb_trail_clean_up_agent::clean_up_if_final_prize_resource_deleted(ctx, entity_id);
    }

    if let Some(mut count) = ctx.db.resource_count().resource_id().find(&resource_id) {
        count.num_in_world -= 1;
        ctx.db.resource_count().resource_id().update(count);
    }

    ctx.db.light_source_state().entity_id().delete(&entity_id);
    delete_footprint(ctx, entity_id);
    AttachedHerdsState::delete(ctx, entity_id);
    delete_entity(ctx, entity_id);
}
