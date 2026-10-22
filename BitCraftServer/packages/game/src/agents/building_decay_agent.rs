use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents, building_desc, building_state, claim_state,
    game::{
        dimensions,
        entities::building_state::BuildingState,
        reducer_helpers::{building_helpers::delete_building, floating_platform_helpers},
    },
    health_state, location_state,
    messages::authentication::ServerIdentity,
    parameters_desc,
};

#[spacetimedb::table(accessor = building_decay_loop_timer, scheduled(building_decay_agent_loop, at = scheduled_at))]
pub struct BuildingDecayLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(&0).unwrap().building_decay_tick_millis as u64;
    let mut count = 0;
    for mut timer in ctx.db.building_decay_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(tick_length).into();
        ctx.db.building_decay_loop_timer().scheduled_id().update(timer);
        log::info!("building decay agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one BuildingDecayLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    // schedule first tick
    let tick_length = ctx.db.parameters_desc().version().find(&0).unwrap().building_decay_tick_millis as u64;
    ctx.db
        .building_decay_loop_timer()
        .try_insert(BuildingDecayLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[shared_table_reducer]
#[spacetimedb::reducer]
fn building_decay_agent_loop(ctx: &ReducerContext, _timer: BuildingDecayLoopTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        log::error!("Unauthorized access to building_decay agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    // Decay every building outside a claim, draining their hitpoints.
    update_claim_building_health(
        ctx,
        &ctx.db
            .building_state()
            .claim_entity_id()
            .filter(0 as u64)
            .map(
                |BuildingState {
                     entity_id,
                     building_description_id,
                     ..
                 }| (entity_id, building_description_id),
            )
            .collect(),
        true,
    );

    // Spend supplies for each claim based on their tiles count
    for claim in ctx.db.claim_state().neutral().filter(false) {
        let claim_entity_id = claim.entity_id;
        let claim_local = claim.local_state(ctx);
        if claim_local.supplies > 0 {
            let maintenance = claim_local.full_maintenance(ctx).min(claim_local.supplies as f32);
            let _ = claim_local.update_supplies_and_commit(ctx, -maintenance, false);

            // Filled claims heal their buildings
            update_claim_building_health(
                ctx,
                &ctx.db
                    .building_state()
                    .claim_entity_id()
                    .filter(claim_entity_id)
                    .map(
                        |BuildingState {
                             entity_id,
                             building_description_id,
                             ..
                         }| (entity_id, building_description_id),
                    )
                    .collect(),
                false,
            );
        } else {
            // Empty claims have their buildings decaying
            update_claim_building_health(
                ctx,
                &ctx.db
                    .building_state()
                    .claim_entity_id()
                    .filter(claim_entity_id)
                    .map(
                        |BuildingState {
                             entity_id,
                             building_description_id,
                             ..
                         }| (entity_id, building_description_id),
                    )
                    .collect(),
                true,
            );
        }
    }
}

fn update_claim_building_health(ctx: &ReducerContext, buildings: &Vec<(u64, i32)>, decrease_health: bool) {
    for (entity_id, building_description_id) in buildings {
        let building_desc = match ctx.db.building_desc().id().find(building_description_id) {
            Some(bd) => bd,
            None => {
                log::error!("Couldn't find BuildingDesc {building_description_id}");
                continue;
            }
        };
        if building_desc.ignore_damage {
            continue;
        }
        let decay = building_desc.decay * if decrease_health { 1.0 } else { -1.0 };
        if decay != 0.0 {
            let coord = match ctx.db.location_state().entity_id().find(entity_id) {
                Some(ls) => ls.coordinates(),
                None => {
                    log::error!("Couldn't find LocationState for building {entity_id}");
                    continue;
                }
            };
            if coord.dimension == dimensions::OVERWORLD {
                building_decay(ctx, *entity_id, decay, building_desc.max_health as f32);
            }
        }
    }
}

fn building_decay(ctx: &ReducerContext, building_entity_id: u64, decay: f32, max_health: f32) {
    // Claimed buildings are already tallied by iterating through every claim description previously

    let mut health_state = match ctx.db.health_state().entity_id().find(&building_entity_id) {
        Some(hs) => hs,
        None => {
            spacetimedb::log::error!("Unable to update building {} health", building_entity_id);
            return;
        }
    };
    let new_health = (health_state.health - decay).clamp(0.0, max_health);
    let health_changed = health_state.health != new_health;
    if health_changed {
        health_state.health = new_health;
        ctx.db.health_state().entity_id().update(health_state);
    }

    if new_health == 0.0 {
        // Floating platforms whose destruction was postponed stay at 0 health, so keep retrying them on every pass
        let is_floating_platform = floating_platform_helpers::platform_cell_of(ctx, building_entity_id).is_some();
        if !health_changed && !is_floating_platform {
            return;
        }
        if is_floating_platform && floating_platform_helpers::is_platform_occupied(ctx, building_entity_id) {
            // Removing it would strand pets or mobs in the water: wait until they're gone (players and deployables don't block it)
            return;
        }
        // Destroy building
        delete_building(ctx, 0, building_entity_id, None, false, true);
    }
}
