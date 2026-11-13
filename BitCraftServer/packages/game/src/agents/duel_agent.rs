use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::game_state::game_state_filters,
    messages::{
        authentication::ServerIdentity,
        components::{duel_state, health_state, signed_in_player_state},
        static_data::parameters_desc,
    },
    params,
};

#[spacetimedb::table(accessor = duel_agent_timer, scheduled(duel_agent_timer_loop, at = scheduled_at))]
pub struct DuelAgentTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let mut count = 0;
    for mut timer in ctx.db.duel_agent_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(1000).into();
        ctx.db.duel_agent_timer().scheduled_id().update(timer);
    }
    if count > 1 {
        log::error!("More than one duel_agent_timer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .duel_agent_timer()
        .try_insert(DuelAgentTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(1000).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn duel_agent_timer_loop(ctx: &ReducerContext, _timer: DuelAgentTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to duel agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let timeout_millis = params!(ctx).duel_out_of_range_grace_period_millis;
    let duel_range = params!(ctx).duel_range;

    for mut duel in ctx.db.duel_state().iter() {
        let duel_coord = game_state_filters::coordinates_float(ctx, duel.entity_id);
        let mut updated = false;

        for i in 0..duel.player_entity_ids.len() {
            let entity_id = duel.player_entity_ids[i];
            // Check if still signed in
            if ctx.db.signed_in_player_state().entity_id().find(entity_id).is_none() {
                updated = true;
                duel.set_loser(ctx, i);
                break;
            }
            // Check if still alive
            let health = ctx.db.health_state().entity_id().find(entity_id).unwrap();
            if health.health <= 0.0 {
                updated = true;
                duel.set_loser(ctx, i);
                break;
            }
            // Check if still in range
            let coord = game_state_filters::coordinates_float(ctx, entity_id);
            let out_of_range = coord.dimension != duel_coord.dimension || coord.distance_to(duel_coord) > duel_range;

            updated |= duel.update_out_of_range_timestamp(ctx, i, out_of_range);
            if out_of_range
                && duel.timed_out(ctx, i, timeout_millis) {
                    updated = true;
                    duel.set_loser(ctx, i);
                    break;
                }
        }
        if updated {
            ctx.db.duel_state().entity_id().update(duel);
        }
    }
}
