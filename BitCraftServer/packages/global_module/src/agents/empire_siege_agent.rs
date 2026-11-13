use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table, Timestamp};
use std::{collections::HashMap, time::Duration};

use crate::{
    agents,
    messages::{authentication::ServerIdentity, empire_shared::*},
    parameters_desc,
};

fn cost_for_next_tick(
    empire_siege_tick: i32,
    empire_siege_raise_pct: f32,
    empire_siege_delay_ticks: i32,
    start_time: Timestamp,
    now: Timestamp,
) -> i32 {
    let secs = now.duration_since(start_time).unwrap().as_secs() as i32;
    let cycles = secs / empire_siege_tick;
    if cycles < empire_siege_delay_ticks {
        return 0;
    }

    let cycles_after_delay = cycles - empire_siege_delay_ticks + 1;
    let cost = f32::round(cycles_after_delay as f32 * empire_siege_raise_pct);
    cost as i32
}

#[spacetimedb::table(accessor = empire_siege_loop_timer, scheduled(empire_siege_agent_loop, at = scheduled_at))]
pub struct EmpireSiegeLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub first_tick: bool,
}

pub fn update_timer(ctx: &ReducerContext) {
    for timer in ctx.db.empire_siege_loop_timer().iter() {
        ctx.db.empire_siege_loop_timer().scheduled_id().delete(timer.scheduled_id);
    }
    init(ctx);
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().empire_siege_tick_millis as u64;
    let now = (ctx.timestamp.to_micros_since_unix_epoch() / 1000) as u64;
    let num_cycles = now / tick_length;
    let next_tick = (num_cycles + 1) * tick_length;
    let next_tick = Timestamp::from_duration_since_unix_epoch(Duration::from_millis(next_tick));

    ctx.db
        .empire_siege_loop_timer()
        .try_insert(EmpireSiegeLoopTimer {
            scheduled_id: 0,
            scheduled_at: next_tick.into(),
            first_tick: true,
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn empire_siege_agent_loop(ctx: &ReducerContext, timer: EmpireSiegeLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to empire_siege_agent");
        return;
    }

    if timer.first_tick {
        // schedule repeating after first tick
        let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().empire_siege_tick_millis as u64;

        ctx.db
            .empire_siege_loop_timer()
            .try_insert(EmpireSiegeLoopTimer {
                scheduled_id: 0,
                scheduled_at: Duration::from_millis(tick_length).into(),
                first_tick: false,
            })
            .ok()
            .unwrap();
    }

    if !agents::should_run(ctx) {
        return;
    }

    let parameters = ctx.db.parameters_desc().version().find(0).unwrap();
    let empire_siege_tick = parameters.empire_siege_tick_millis / 1000;
    let empire_siege_raise_pct = parameters.empire_siege_raise_pct;
    let empire_siege_delay_ticks = parameters.empire_siege_delay_ticks;
    let now = ctx.timestamp;

    let mut ended_sieges: HashMap<u64, u64> = HashMap::new(); // BuildingEntityId, Attacking Empire

    // Spend energy for each empire node based on their upkeep
    for mut siege in ctx.db.empire_node_siege_state().active().filter(true) {
        let siege_empire_entity_id = siege.empire_entity_id;
        let building_entity_id = siege.building_entity_id;

        let cost = cost_for_next_tick(
            empire_siege_tick,
            empire_siege_raise_pct,
            empire_siege_delay_ticks,
            siege.start_timestamp.unwrap(),
            now,
        );
        if cost == 0 {
            continue;
        }
        if cost > siege.energy {
            let drain = cost - siege.energy;
            let mut empire_node = ctx.db.empire_node_state().entity_id().find(building_entity_id).unwrap();
            if siege.energy > 0 {
                siege.energy = 0;
                EmpireNodeSiegeState::update_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
            }

            if empire_node.empire_entity_id == siege_empire_entity_id {
                // Defender sieges lose some durability
                if empire_node.energy < drain {
                    if empire_node.energy > 0 {
                        empire_node.energy = 0;
                        EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
                    }
                    // Defender siege ends when tower runs out of supplies
                    ended_sieges.entry(building_entity_id).or_insert_with(|| {
                        EmpireNodeSiegeState::get_attacking_siege(ctx, building_entity_id)
                            .unwrap()
                            .empire_entity_id
                    });
                } else {
                    empire_node.energy -= drain;
                    EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
                }
            } else {
                // Attacker siege end
                ended_sieges.insert(building_entity_id, siege_empire_entity_id);
            }
        } else {
            siege.energy -= cost;
            EmpireNodeSiegeState::update_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
    }

    for (building_entity_id, sieging_empire_entity_id) in ended_sieges {
        EmpireNodeSiegeState::end_siege(ctx, building_entity_id, sieging_empire_entity_id);
    }
}
