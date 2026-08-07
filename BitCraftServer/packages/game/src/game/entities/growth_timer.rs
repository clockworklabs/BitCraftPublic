use std::time::Duration;

use spacetimedb::{log, rand::Rng, ReducerContext, ScheduleAt, Table};

use crate::{
    game::autogen::_delete_entity::delete_entity,
    inter_module::system_chat_broadcast::{sytem_chat_broadcast_timer, SystemChatBroadcastTimer},
    location_state,
    messages::{
        authentication::ServerIdentity,
        components::{PlaceableState, ResourceState},
        generic::world_region_state,
        static_data::{PlaceableGrowthDesc, ResourceGrowthRecipeDesc},
    },
    placeable_growth_desc, placeable_state, resource_growth_recipe_desc, resource_state, unwrap_or_return,
};

#[spacetimedb::table(name = resource_growth_timer, public, scheduled(resource_growth_scheduled, at = scheduled_at))]
#[spacetimedb::table(name = placeable_growth_timer, public, scheduled(placeable_growth_scheduled, at = scheduled_at))]
pub struct GrowthTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
    #[unique]
    pub entity_id: u64,
    pub growth_recipe_id: i32,
}

pub fn insert_resource_growth_timer(
    ctx: &ReducerContext,
    entity_id: u64,
    growth: &ResourceGrowthRecipeDesc,
) -> Result<GrowthTimer, String> {
    let end_timestamp = ctx.timestamp + Duration::from_secs_f32(get_duration(ctx, growth.time.clone()));
    let timer = ctx.db.resource_growth_timer().try_insert(GrowthTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Time(end_timestamp),
        entity_id,
        growth_recipe_id: growth.id,
    })?;
    broadcast_growth(ctx, entity_id, growth, end_timestamp);
    Ok(timer)
}

pub fn insert_placeable_growth_timer(
    ctx: &ReducerContext,
    entity_id: u64,
    growth: &PlaceableGrowthDesc,
) -> Result<GrowthTimer, String> {
    let end_timestamp = ctx.timestamp + Duration::from_secs_f32(get_duration(ctx, growth.time.clone()));
    Ok(ctx.db.placeable_growth_timer().try_insert(GrowthTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Time(end_timestamp),
        entity_id,
        growth_recipe_id: growth.id,
    })?)
}

pub fn delete_resource_growth_timer(ctx: &ReducerContext, entity_id: u64) {
    ctx.db.resource_growth_timer().entity_id().delete(&entity_id);
}

pub fn delete_placeable_growth_timer(ctx: &ReducerContext, entity_id: u64) {
    ctx.db.placeable_growth_timer().entity_id().delete(&entity_id);
}

#[spacetimedb::reducer]
fn resource_growth_scheduled(ctx: &ReducerContext, timer: GrowthTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let resource = match ctx.db.resource_state().entity_id().find(&timer.entity_id) {
        Some(resource) => resource,
        None => return Ok(()),
    };
    let location = match ctx.db.location_state().entity_id().find(&timer.entity_id) {
        Some(location) => location,
        None => {
            log::error!("Resource growth timer {} is missing location", timer.entity_id);
            return Ok(());
        }
    };
    let recipe = match ctx.db.resource_growth_recipe_desc().id().find(&timer.growth_recipe_id) {
        Some(recipe) => recipe,
        None => {
            log::error!(
                "Resource growth timer {} references missing recipe {}",
                timer.entity_id,
                timer.growth_recipe_id
            );
            return Ok(());
        }
    };

    let coordinates = location.coordinates();
    let direction = resource.direction_index;
    resource.despawn_growing(ctx);

    let should_spawn = if recipe.grown_resource_chance >= 1.0 {
        true
    } else if recipe.grown_resource_chance <= 0.0 {
        false
    } else {
        ctx.rng().gen_range(0.0..=1.0) <= recipe.grown_resource_chance
    };

    if recipe.grown_resource_id != 0
        && should_spawn
        && !ResourceState::spawn_in_radius_band_with_fallback(
            ctx,
            recipe.grown_resource_id,
            coordinates,
            direction,
            recipe.grown_resource_min_radius,
            recipe.grown_resource_max_radius,
        )
    {
        log::error!(
            "Unable to grow resource {} into {}",
            timer.entity_id,
            recipe.grown_resource_id
        );
    }

    Ok(())
}

#[spacetimedb::reducer]
fn placeable_growth_scheduled(ctx: &ReducerContext, timer: GrowthTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let placeable = match ctx.db.placeable_state().entity_id().find(&timer.entity_id) {
        Some(placeable) => placeable,
        None => return Ok(()),
    };
    let location = match ctx.db.location_state().entity_id().find(&timer.entity_id) {
        Some(location) => location,
        None => {
            log::error!("Placeable growth timer {} is missing location", timer.entity_id);
            return Ok(());
        }
    };
    let recipe = match ctx.db.placeable_growth_desc().id().find(&timer.growth_recipe_id) {
        Some(recipe) => recipe,
        None => {
            log::error!(
                "Placeable growth timer {} references missing recipe {}",
                timer.entity_id,
                timer.growth_recipe_id
            );
            return Ok(());
        }
    };

    let coordinates = location.coordinates();
    let direction = placeable.direction_index;
    let owner_entity_id = placeable.owner_entity_id;
    delete_entity(ctx, placeable.entity_id);

    if let Some(outcome) = PlaceableState::pick_growth_outcome(ctx, recipe.outcomes_v2) {
        if let Err(err) = PlaceableState::spawn_growth_outcome_with_fallback(
            ctx,
            owner_entity_id,
            coordinates,
            direction,
            &outcome,
        ) {
            log::error!(
                "Unable to grow placeable {} into {}: {}",
                timer.entity_id,
                outcome.placeable_id,
                err
            );
        }
    }

    Ok(())
}

fn get_duration(ctx: &ReducerContext, time: Vec<f32>) -> f32 {
    if time.len() <= 1 {
        return time.first().copied().unwrap_or(0.0);
    }
    ctx.rng().gen_range(time[0]..=time[1])
}

fn broadcast_growth(
    ctx: &ReducerContext,
    entity_id: u64,
    growth: &ResourceGrowthRecipeDesc,
    end_timestamp: spacetimedb::Timestamp,
) {
    const INACTIVE_HEXITE_SEALED_CHEST_GROWTH_ID: i32 = 1633012503;
    if growth.id != INACTIVE_HEXITE_SEALED_CHEST_GROWTH_ID {
        return;
    }

    let location = unwrap_or_return!(ctx.db.location_state().entity_id().find(entity_id), "Unknown location")
        .coordinates()
        .parent_large_tile()
        .to_offset_coordinates();
    let region = unwrap_or_return!(ctx.db.world_region_state().id().find(0), "Unknown region");

    for (duration, description) in [
        (Duration::from_hours(1), "1 hour"),
        (Duration::from_mins(15), "15 minutes"),
        (Duration::from_mins(5), "5 minutes"),
    ] {
        ctx.db.sytem_chat_broadcast_timer().insert(SystemChatBroadcastTimer {
            scheduled_id: 0,
            scheduled_at: ScheduleAt::Time(end_timestamp - duration),
            message: format!(
                "The (res={{0}}) in Region {{1}} at (coord={{2}},{{3}}) is preparing to unlock in {{4}}.|~{}|~{}|~{}|~{}|~{}",
                growth.resource_id, region.region_index, location.z, location.x, description
            ),
        });
    }
}
