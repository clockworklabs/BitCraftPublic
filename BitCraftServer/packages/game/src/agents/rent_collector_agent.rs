use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents, claim_state,
    game::{game_state, reducer_helpers::timer_helpers::now_plus_secs},
    messages::{authentication::ServerIdentity, components::claim_local_state},
    parameters_desc, rent_state,
};

const SECONDS_IN_A_DAY: i32 = 24 * 60 * 60;

#[spacetimedb::table(accessor = rent_collector_loop_timer, scheduled(rent_collector_agent_loop, at = scheduled_at))]
pub struct RentCollectorLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub first_tick: bool,
}

pub fn schedule_first_tick(ctx: &ReducerContext) {
    let tick_time_of_day = ctx.db.parameters_desc().version().find(0).unwrap().rent_collection_time_of_day;
    let daily_timestamp_tick = (tick_time_of_day * 60.0 * 60.0) as i32;
    let seconds_elapsed = game_state::unix(ctx.timestamp);
    let start_of_current_day_timestamp = (seconds_elapsed / SECONDS_IN_A_DAY) * SECONDS_IN_A_DAY;
    let mut next_tick = start_of_current_day_timestamp + daily_timestamp_tick;
    if next_tick < seconds_elapsed {
        next_tick += SECONDS_IN_A_DAY;
    }
    let time_until_next_day = next_tick - seconds_elapsed;
    ctx.db
        .rent_collector_loop_timer()
        .try_insert(RentCollectorLoopTimer {
            scheduled_id: 0,
            first_tick: true,
            scheduled_at: now_plus_secs(time_until_next_day as u64, ctx.timestamp),
        })
        .ok()
        .unwrap();
}

pub fn init(ctx: &ReducerContext) {
    schedule_first_tick(ctx);
}

#[spacetimedb::reducer]
fn rent_collector_agent_loop(ctx: &ReducerContext, timer: RentCollectorLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to rent_collector agent");
        return;
    }

    if timer.first_tick {
        // schedule repeating after first tick
        ctx.db
            .rent_collector_loop_timer()
            .try_insert(RentCollectorLoopTimer {
                first_tick: false,
                scheduled_id: 0,
                scheduled_at: Duration::from_secs(SECONDS_IN_A_DAY as u64).into(),
            })
            .ok()
            .unwrap();
    }

    if !agents::should_run(ctx) {
        return;
    }

    // Collect from each rent
    for rent_state in ctx.db.rent_state().iter() {
        if rent_state.daily_rent > 0 && rent_state.eviction_timestamp.is_none() {
            let able_to_pay_rent = rent_state.paid_rent >= rent_state.daily_rent;
            if able_to_pay_rent || !rent_state.defaulted {
                let claim_entity_id = rent_state.claim_entity_id;

                let mut rent_state = rent_state.clone();
                if able_to_pay_rent {
                    rent_state.defaulted = false;
                    rent_state.paid_rent -= rent_state.daily_rent;

                    // There should always be a claim associated to a rent
                    if let Some(claim_entity) = ctx.db.claim_state().entity_id().find(claim_entity_id) {
                        let mut claim_entity = claim_entity.local_state(ctx);
                        claim_entity.treasury += rent_state.daily_rent;
                        ctx.db.claim_local_state().entity_id().update(claim_entity);
                    }
                } else {
                    rent_state.defaulted = true;
                }
                ctx.db.rent_state().entity_id().update(rent_state);
            }
        }
    }
}
