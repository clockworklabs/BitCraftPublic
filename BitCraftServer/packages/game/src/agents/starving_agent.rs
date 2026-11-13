use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::{reducer_helpers::health_helpers::update_health_and_check_death, terrain_chunk::TerrainChunkCache},
    health_state,
    messages::authentication::ServerIdentity,
    parameters_desc, signed_in_player_state, starving_player_state, ParametersDesc,
};

#[spacetimedb::table(accessor = starving_loop_timer, scheduled(starving_agent_loop, at = scheduled_at))]
pub struct StarvingLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    let mut count = 0;
    for mut timer in ctx.db.starving_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(params.starving_tick_millis as u64).into();
        ctx.db.starving_loop_timer().scheduled_id().update(timer);
        log::info!("starving agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one Starving Agent running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let params: ParametersDesc = ctx.db.parameters_desc().version().find(0).unwrap();
    ctx.db
        .starving_loop_timer()
        .try_insert(StarvingLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(params.starving_tick_millis as u64).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn starving_agent_loop(ctx: &ReducerContext, _timer: StarvingLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to starving agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let starving_damage = ctx.db.parameters_desc().version().find(0).unwrap().starving_damage.abs();

    reduce(ctx, &starving_damage);
}

fn reduce(ctx: &ReducerContext, starving_damage: &f32) {
    let mut terrain_cache = TerrainChunkCache::empty();

    for starving_player_state in ctx.db.starving_player_state().iter() {
        let player_entity_id = starving_player_state.entity_id;

        if ctx.db.signed_in_player_state().entity_id().find(player_entity_id).is_none() {
            continue;
        }

        let mut health_state = ctx.db.health_state().entity_id().find(player_entity_id).unwrap();
        if health_state.health > 0.0 {
            health_state.add_health_delta(-starving_damage, ctx.timestamp);
            update_health_and_check_death(ctx, &mut terrain_cache, health_state, player_entity_id, None);
        }
    }
}
