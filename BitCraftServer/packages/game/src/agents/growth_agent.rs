use std::time::Duration;

use spacetimedb::rand::Rng;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents, growth_state, location_state,
    messages::{
        authentication::ServerIdentity,
        components::{PlaceableState, ResourceState},
    },
    parameters_desc, placeable_growth_desc, placeable_state, resource_growth_recipe_desc, resource_state,
};

#[spacetimedb::table(name = growth_loop_timer, scheduled(growth_agent_loop, at = scheduled_at))]
pub struct GrowthLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = ctx
        .db
        .parameters_desc()
        .version()
        .find(&0)
        .unwrap()
        .resource_growth_tick_rate_milliseconds as u64;
    let mut count = 0;
    for mut timer in ctx.db.growth_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(tick_length).into();
        ctx.db.growth_loop_timer().scheduled_id().update(timer);
        log::info!("resource growth agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one GrowthLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = ctx
        .db
        .parameters_desc()
        .version()
        .find(&0)
        .unwrap()
        .resource_growth_tick_rate_milliseconds as u64;
    ctx.db
        .growth_loop_timer()
        .try_insert(GrowthLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn growth_agent_loop(ctx: &ReducerContext, _timer: GrowthLoopTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        log::error!("Unauthorized access to growth agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let now = ctx.timestamp;
    for grown in ctx.db.growth_state().iter().filter(|ga| ga.end_timestamp < now) {
        if let Some(deposit) = ctx.db.resource_state().entity_id().find(&grown.entity_id) {
            // Only evolve if the resource is there (which it should be, otherwise the growth component would have been deleted)
            // Stored the recipe instead of the id in case someday we grow non-resources or have
            // a grown entity evolving into a different type of entity
            let loc = match ctx.db.location_state().entity_id().find(&grown.entity_id) {
                Some(l) => l,
                None => {
                    spacetimedb::log::error!("GrowthState {} is missing location", grown.entity_id);
                    continue;
                }
            };
            let coordinates = loc.coordinates();
            let direction = deposit.direction_index;
            deposit.despawn_self(ctx);

            let recipe = ctx.db.resource_growth_recipe_desc().id().find(&grown.growth_recipe_id).unwrap();
            let target_resource_id = recipe.grown_resource_id;
            let should_spawn = if recipe.grown_resource_chance >= 1.0 {
                true
            } else if recipe.grown_resource_chance <= 0.0 {
                false
            } else {
                ctx.rng().gen_range(0.0..=1.0) <= recipe.grown_resource_chance
            };

            if target_resource_id != 0
                && should_spawn
                && !ResourceState::spawn_in_radius_band_with_fallback(
                    ctx,
                    target_resource_id,
                    coordinates,
                    direction,
                    recipe.grown_resource_min_radius,
                    recipe.grown_resource_max_radius,
                )
            {
                log::error!("Unable to grow resource {} into {}", grown.entity_id, target_resource_id);
            }
        } else if let Some(placeable) = ctx.db.placeable_state().entity_id().find(&grown.entity_id) {
            let loc = match ctx.db.location_state().entity_id().find(&grown.entity_id) {
                Some(l) => l,
                None => {
                    spacetimedb::log::error!("GrowthState {} is missing location", grown.entity_id);
                    continue;
                }
            };
            let coordinates = loc.coordinates();
            let direction = placeable.direction_index;
            let owner_entity_id = placeable.owner_entity_id;
            placeable.despawn(ctx);

            let recipe = ctx.db.placeable_growth_desc().id().find(&grown.growth_recipe_id).unwrap();
            if let Some(outcome) = PlaceableState::pick_growth_outcome(ctx, recipe.outcomes_v2) {
                if let Err(err) = PlaceableState::spawn_growth_outcome_with_fallback(
                    ctx,
                    owner_entity_id,
                    coordinates,
                    direction,
                    &outcome,
                ) {
                    log::error!("Unable to grow placeable {} into {}: {}", grown.entity_id, outcome.placeable_id, err);
                }
            }
        }
    }
}
