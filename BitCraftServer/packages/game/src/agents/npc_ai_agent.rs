use spacetimedb::rand::Rng;
use spacetimedb::{duration, log, ReducerContext, Table};
use std::collections::HashMap;

use crate::game::coordinates::*;
use crate::messages::authentication::ServerIdentity;
use crate::{
    agents,
    game::location_cache::RuinsEntityValuePair,
    messages::{
        components::NpcState,
        static_data::{BuildingSpawnDesc, NpcType},
    },
};
use crate::{building_state, location_cache, npc_desc, npc_state};

#[spacetimedb::table(accessor = npc_ai_loop_timer, scheduled(npc_ai_agent_loop, at = scheduled_at))]
pub struct NpcAiLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .npc_ai_loop_timer()
        .try_insert(NpcAiLoopTimer {
            scheduled_id: 0,
            scheduled_at: duration!(300s).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn npc_ai_agent_loop(ctx: &ReducerContext, _timer: NpcAiLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to npc_ai agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let mut broken_ruins: Vec<u64> = Vec::new();
    let mut location_cache = ctx.db.location_cache().version().find(0).unwrap();
    let traveler_ruins = &location_cache.traveler_ruins;
    let mut required_spawns: HashMap<NpcType, usize> = HashMap::new();

    if !traveler_ruins.is_empty() {
        // Identify how many npc instances are required for each type based on population in csv
        for npc in ctx.db.npc_desc().iter() {
            required_spawns.insert(
                NpcType::to_enum(npc.npc_type),
                (npc.population * traveler_ruins.len() as f32) as usize,
            );
        }
    }

    let mut free_ruins = traveler_ruins.clone();
    let mut occupied_ruins = Vec::new();
    let mut acting_npcs = Vec::new();

    let time = ctx.timestamp;

    // Divide ruins between free and occupied based on the current NPC population
    for npc in ctx.db.npc_state().iter() {
        let ruin_entity_id = npc.building_entity_id;
        let npc_type = npc.npc_type;
        let is_traveling_npc = npc.traveling;

        if time.duration_since(npc.next_action_timestamp).is_some() {
            // This npc needs to act.
            acting_npcs.push(npc);
        }

        if is_traveling_npc {
            if let Some(index) = free_ruins.iter().position(|kvp| kvp.entity_id == ruin_entity_id) {
                occupied_ruins.push(free_ruins[index]);
                free_ruins.remove(index);
            }
            // Need 1 less spawn for that npc type
            if let Some(count) = required_spawns.get(&npc_type) {
                required_spawns.insert(npc_type, if *count > 0 { *count - 1 } else { 0 });
            }
        }
    }

    // Attempt moving all the npcs that are past their timestamp
    for mut moving_npc in acting_npcs.into_iter() {
        if moving_npc.traveling {
            let moving_npc_entity_id = moving_npc.entity_id;
            if let Some(target_index) = pick_free_ruin_index(ctx, moving_npc_entity_id, &free_ruins, &occupied_ruins) {
                // occupies / locks next building
                let target_ruin = free_ruins[target_index];
                let target_ruin_entity_id = target_ruin.entity_id;
                if let Some(building) = ctx.db.building_state().entity_id().find(target_ruin_entity_id) {
                    occupied_ruins.push(target_ruin);
                    free_ruins.remove(target_index);

                    moving_npc.teleport(ctx, &building);

                    // update npc building
                    let npc_building = moving_npc.building_entity_id;
                    moving_npc.building_entity_id = target_ruin_entity_id;
                    ctx.db.npc_state().entity_id().update(moving_npc);

                    // free occupied building
                    if let Some(source_index) = occupied_ruins.iter().position(|kvp| kvp.entity_id == npc_building) {
                        free_ruins.push(occupied_ruins[source_index]);
                        occupied_ruins.remove(source_index);
                    }
                } else {
                    // Building no longer exists. This shouldn't happen, but it's been known to happen thanks to players deconstructing ruins.
                    free_ruins.remove(target_index);
                    broken_ruins.push(target_ruin_entity_id);
                }
            }
        } else {
            let moving_npc_entity_id = moving_npc.entity_id;
            if ctx.db.building_state().entity_id().find(moving_npc.building_entity_id).is_some() {
                // delete current trade orders
                moving_npc.delete_trade_orders(ctx);
                moving_npc.create_trade_orders(ctx);

                // reset acting timestamp
                moving_npc.next_action_timestamp = NpcState::get_next_timestamp(ctx, moving_npc.npc_type);
                ctx.db.npc_state().entity_id().update(moving_npc);
            } else {
                // delete this npc that tragically lost its camp
                ctx.db.npc_state().entity_id().delete(moving_npc_entity_id);
            }
        }
    }

    // Attempt spawning all remaining npcs
    for (spawn_type, count) in required_spawns.iter() {
        for _i in 0..*count {
            if free_ruins.is_empty() {
                log::error!("Not enough free ruins to spawn all npcs. Make sure the sum of all population percentages don't exceed 1.0 in the csv file.");
                break;
            }
            let ruin_ix = ctx.rng().gen_range(0..free_ruins.len());
            let ruin = free_ruins[ruin_ix];
            if let Some(building) = ctx.db.building_state().entity_id().find(ruin.entity_id) {
                let traveler_coords = BuildingSpawnDesc::get_traveler_spawn_coordinates(
                    ctx,
                    building.building_description_id,
                    &ruin.coordinates,
                    building.direction_index,
                );
                let offset = OffsetCoordinatesSmall::from(traveler_coords);
                let traveler_direction =
                    BuildingSpawnDesc::get_traveler_direction(ctx, building.building_description_id, building.direction_index);

                occupied_ruins.push(free_ruins[ruin_ix]);
                free_ruins.remove(ruin_ix);

                NpcState::spawn(ctx, *spawn_type, traveler_direction, ruin.entity_id, offset, true);
            } else {
                // Building no longer exists. This shouldn't happen, but it's been known to happen thanks to players deconstructing ruins.
                free_ruins.remove(ruin_ix);
                broken_ruins.push(ruin.entity_id);
            }
        }
    }

    if !broken_ruins.is_empty() {
        for broken_ruin_entity_id in broken_ruins {
            if let Some(idx) = location_cache
                .traveler_ruins
                .iter()
                .position(|r| r.entity_id == broken_ruin_entity_id)
            {
                location_cache.traveler_ruins.remove(idx);
            }
            if let Some(idx) = location_cache.all_ruins.iter().position(|r| r.entity_id == broken_ruin_entity_id) {
                location_cache.all_ruins.remove(idx);
            }
            log::warn!("Removing ancient ruin {} from location cache", broken_ruin_entity_id);
        }
        ctx.db.location_cache().version().update(location_cache);
    }
}

fn pick_free_ruin_index(
    ctx: &ReducerContext,
    moving_npc_entity_id: u64,
    free_ruins: &[RuinsEntityValuePair],
    occupied_ruins: &[RuinsEntityValuePair],
) -> Option<usize> {
    let npc = ctx.db.npc_state().entity_id().find(moving_npc_entity_id).unwrap();

    if let Some(index) = occupied_ruins.iter().position(|kvp| kvp.entity_id == npc.building_entity_id) {
        let start = occupied_ruins[index];

        // find the 3 closest ruins that were not visited recently
        let mut sorted_ruins = free_ruins.to_owned();
        sorted_ruins.sort_by(|r1, r2| {
            r1.coordinates
                .distance_to(start.coordinates)
                .cmp(&r2.coordinates.distance_to(start.coordinates))
        });

        let sorted_ruins_copy = sorted_ruins.clone();
        sorted_ruins.retain(|r| !npc.previous_buildings.contains(&r.entity_id));

        if sorted_ruins.is_empty() {
            // All free buildings were recently visited. Pick one of these then (better than none).
            sorted_ruins = sorted_ruins_copy;
        }

        let max = 3.min(sorted_ruins.len());

        if max == 0 {
            // No free ruin
            return None;
        }

        // pick randomly one of the three
        let ruin = sorted_ruins[ctx.rng().gen_range(0..max)];
        Some(free_ruins.iter().position(|kvp| kvp.entity_id == ruin.entity_id).unwrap())
    } else {
        None
    }
}
