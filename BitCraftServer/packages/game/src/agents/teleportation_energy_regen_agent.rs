use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::messages::components::teleportation_energy_state;
use crate::{
    agents,
    messages::{authentication::ServerIdentity, static_data::*},
};

#[spacetimedb::table(accessor = teleportation_energy_regen_loop_timer, scheduled(teleportation_energy_regen_agent_loop, at = scheduled_at))]
pub struct TeleportationEnergyRegenLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    let mut count = 0;
    for mut timer in ctx.db.teleportation_energy_regen_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(params.teleportation_energy_regen_tick_millis as u64).into();
        ctx.db.teleportation_energy_regen_loop_timer().scheduled_id().update(timer);
        log::info!("player regen agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one TeleportationEnergyRegenLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    ctx.db
        .teleportation_energy_regen_loop_timer()
        .try_insert(TeleportationEnergyRegenLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(params.teleportation_energy_regen_tick_millis as u64).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn teleportation_energy_regen_agent_loop(ctx: &ReducerContext, _timer: TeleportationEnergyRegenLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to teleportation_energy_regen agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    for mut teleportation_energy_state in ctx.db.teleportation_energy_state().iter() {
        if teleportation_energy_state.add_energy_regen(ctx) {
            teleportation_energy_state.update(ctx);
        }
    }
}
