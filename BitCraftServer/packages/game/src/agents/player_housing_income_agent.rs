use std::{collections::HashMap, time::Duration};

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents, claim_local_state,
    game::{game_state, reducer_helpers::timer_helpers::now_plus_secs},
    messages::{
        authentication::ServerIdentity,
        components::{building_state, player_housing_state, ClaimLocalState},
        static_data::{building_desc, BuildingFunction},
    },
    parameters_desc,
};

const SECONDS_IN_A_DAY: i32 = 24 * 60 * 60;

#[spacetimedb::table(accessor = player_housing_income_loop_timer, scheduled(player_housing_income_agent_loop, at = scheduled_at))]
pub struct PlayerHousingIncomeLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub first_tick: bool,
}

pub fn schedule_first_tick(ctx: &ReducerContext) {
    let tick_time_of_day = ctx
        .db
        .parameters_desc()
        .version()
        .find(0)
        .unwrap()
        .player_housing_income_time_of_day;
    let daily_timestamp_tick = (tick_time_of_day * 60.0 * 60.0) as i32;
    let seconds_elapsed = game_state::unix(ctx.timestamp);
    let start_of_current_day_timestamp = (seconds_elapsed / SECONDS_IN_A_DAY) * SECONDS_IN_A_DAY;
    let mut next_tick = start_of_current_day_timestamp + daily_timestamp_tick;
    if next_tick < seconds_elapsed {
        next_tick += SECONDS_IN_A_DAY;
    }
    let time_until_next_day = next_tick - seconds_elapsed;
    ctx.db
        .player_housing_income_loop_timer()
        .try_insert(PlayerHousingIncomeLoopTimer {
            scheduled_id: 0,
            first_tick: true,
            scheduled_at: now_plus_secs(time_until_next_day as u64, ctx.timestamp), // not repeating
        })
        .ok()
        .unwrap();
}

pub fn init(ctx: &ReducerContext) {
    schedule_first_tick(ctx);
}

#[spacetimedb::reducer]
fn player_housing_income_agent_loop(ctx: &ReducerContext, timer: PlayerHousingIncomeLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to player_housing_income agent");
        return;
    }

    if timer.first_tick {
        // schedule repeating after first tick
        ctx.db
            .player_housing_income_loop_timer()
            .try_insert(PlayerHousingIncomeLoopTimer {
                first_tick: false,
                scheduled_id: 0,
                scheduled_at: Duration::from_secs(SECONDS_IN_A_DAY as u64).into(), // Repeating
            })
            .ok()
            .unwrap();
    }

    if !agents::should_run(ctx) {
        return;
    }

    // Collect from each rent
    let mut buildings_income: HashMap<u64, (u64, u32)> = HashMap::new(); // Buiding Entity Id => (Claim Entity Id, income)
    let mut claims: HashMap<u64, ClaimLocalState> = HashMap::new(); // Claim Entity Id => Claim

    for player_housing in ctx.db.player_housing_state().iter() {
        let (claim_entity_id, income) = buildings_income.entry(player_housing.entrance_building_entity_id).or_insert({
            if let Some(building) = ctx.db.building_state().entity_id().find(player_housing.entrance_building_entity_id) {
                if let Some(claim) = ctx.db.claim_local_state().entity_id().find(building.claim_entity_id) {
                    let desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
                    let income = BuildingFunction::player_housing_income(&desc);
                    let claim_entity_id = claim.entity_id;
                    if income > 0 {
                        // Only keep track of updated claims if the building generates any income
                        claims.entry(claim_entity_id).or_insert(claim);
                    }
                    (claim_entity_id, income)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        });
        if *income > 0 {
            claims.entry(*claim_entity_id).and_modify(|c| c.treasury += *income);
        }
    }

    for (_claim_entity_id, claim) in claims {
        ctx.db.claim_local_state().entity_id().update(claim);
    }
}
