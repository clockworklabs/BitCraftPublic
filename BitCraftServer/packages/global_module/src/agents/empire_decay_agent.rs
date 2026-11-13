use std::{collections::HashSet, time::Duration};

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    messages::{authentication::ServerIdentity, empire_shared::*},
    parameters_desc,
};

#[spacetimedb::table(accessor = empire_decay_loop_timer, scheduled(empire_decay_agent_loop, at = scheduled_at))]
pub struct EmpireDecayLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().empire_decay_tick_millis as u64;
    let mut count = 0;
    for mut timer in ctx.db.empire_decay_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(tick_length).into();
        ctx.db.empire_decay_loop_timer().scheduled_id().update(timer);
        log::info!("empire decay agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one EmpireDecayLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().empire_decay_tick_millis as u64;
    ctx.db
        .empire_decay_loop_timer()
        .try_insert(EmpireDecayLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn empire_decay_agent_loop(ctx: &ReducerContext, _timer: EmpireDecayLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to empire_decay agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    // Spend energy for each empire node based on their upkeep
    let mut updated_empires = HashSet::new();
    for mut empire_node in ctx.db.empire_node_state().active().filter(true) {
        if ctx
            .db
            .empire_node_siege_state()
            .building_entity_id()
            .filter(empire_node.entity_id)
            .any(|siege| siege.active)
        {
            continue;
        }
        if empire_node.energy > empire_node.upkeep {
            empire_node.energy -= empire_node.upkeep;
        } else {
            empire_node.energy = 0;
            empire_node.deactivate(ctx);
            updated_empires.insert(empire_node.empire_entity_id);
        }
        EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    for empire_entity_id in updated_empires {
        EmpireState::update_empire_upkeep(ctx, empire_entity_id);
    }
}
