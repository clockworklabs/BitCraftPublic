use std::time::Duration;

use crate::game::reducer_helpers::health_helpers::update_health_and_check_death;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::authentication::ServerIdentity;
use crate::{
    agents,
    game::entities::buff,
    messages::{components::*, static_data::*},
};
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::table(accessor = active_environment_buff_state, 
    index(accessor = entity_id, btree(columns = [entity_id])), 
    index(accessor = player_buff, btree(columns = [entity_id, buff_id])))]
#[derive(Clone, Debug)]
pub struct ActiveEnvironmentBuffState {
    pub entity_id: u64,
    pub buff_id: i32,
}

#[spacetimedb::table(accessor = environment_debuff_loop_timer, scheduled(environment_debuff_agent_loop, at = scheduled_at))]
pub struct EnvironmentDebuffLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().environment_debuff_tick_millis as u64;
    let mut count = 0;
    for mut timer in ctx.db.environment_debuff_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(tick_length).into();
        ctx.db.environment_debuff_loop_timer().scheduled_id().update(timer);
        log::info!("environment debuff agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one EnvironmentDebuffLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().environment_debuff_tick_millis as u64;
    ctx.db
        .environment_debuff_loop_timer()
        .try_insert(EnvironmentDebuffLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn environment_debuff_agent_loop(ctx: &ReducerContext, _timer: EnvironmentDebuffLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to environment_debuff agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    // TODO: add column in CSV to match character stat and biome with each debuff.
    let debuffs_data: Vec<(i32, u32, CharacterStatType, f32, f32, i32)> = ctx
        .db
        .environment_debuff_desc()
        .iter()
        .map(|d: EnvironmentDebuffDesc| {
            let stat = match d.resistance_type {
                EnvironmentResistanceType::Cold => CharacterStatType::ColdProtection,
                EnvironmentResistanceType::Heat => CharacterStatType::HeatProtection,
            };
            let biome = match d.resistance_type {
                EnvironmentResistanceType::Cold => Biome::SnowyPeaks,
                EnvironmentResistanceType::Heat => Biome::Desert,
            };
            (
                d.buff_id,
                biome as u32,
                stat,
                d.ground_damage as f32,
                d.water_damage as f32,
                d.resistance_level,
            )
        })
        .collect();

    let mut terrain_cache = TerrainChunkCache::empty();

    for player_entity_id in ctx.db.signed_in_player_state().iter().map(|p| p.entity_id) {
        // find active environment debuff
        let coord = ctx
            .db
            .mobile_entity_state()
            .entity_id()
            .find(player_entity_id)
            .unwrap()
            .coordinates();
        if let Some(terrain_chunk) = terrain_cache.get_from_chunk_coordinates(ctx, coord.chunk_coordinates()) {
            let chunk_index = terrain_chunk.get_index(coord.parent_large_tile());
            let current_biome = terrain_chunk.get_biome_index(chunk_index);
            let on_water = terrain_chunk.is_submerged_index(chunk_index);
            for (debuff_id, biome, character_stat, ground_damage, water_damage, resistance_level) in &debuffs_data {
                let debuff_id = *debuff_id;
                let was_debuff_active = ctx
                    .db
                    .active_environment_buff_state()
                    .player_buff()
                    .filter((player_entity_id, debuff_id))
                    .next()
                    .is_some();
                let damage = if on_water { *water_damage } else { *ground_damage };
                let is_debuff_active = current_biome == *biome
                    && damage > 0.0
                    && (CharacterStatsState::get_entity_stat(ctx, player_entity_id, *character_stat) as i32) < *resistance_level;

                if is_debuff_active != was_debuff_active {
                    if is_debuff_active {
                        // Add debuff to player
                        if buff::activate(ctx, player_entity_id, debuff_id, None, None).is_err() {
                            log::error!("Unable to activate debuff {} on entity {}", debuff_id, player_entity_id);
                        }
                        ctx.db.active_environment_buff_state().insert(ActiveEnvironmentBuffState {
                            entity_id: player_entity_id,
                            buff_id: debuff_id,
                        });
                    } else {
                        // Remove debuff from player
                        if buff::deactivate(ctx, player_entity_id, debuff_id).is_err() {
                            log::error!("Unable to deactivate debuff {} on entity {}", debuff_id, player_entity_id);
                        }
                        ctx.db
                            .active_environment_buff_state()
                            .player_buff()
                            .delete((player_entity_id, debuff_id));
                    }
                }
                if is_debuff_active {
                    // Hurt player because of debuff
                    let mut health_state = ctx.db.health_state().entity_id().find(player_entity_id).unwrap();
                    if health_state.health > 0.0 {
                        health_state.add_health_delta(-damage, ctx.timestamp);
                        update_health_and_check_death(ctx, &mut terrain_cache, health_state, player_entity_id, None);
                    }
                }
            }
        }
    }
}
