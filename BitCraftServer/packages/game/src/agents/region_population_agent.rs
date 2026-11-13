use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::messages::generic::{world_region_state, RegionPopulationInfo};
use crate::messages::queue::player_queue_state;
use crate::{agents, messages::authentication::ServerIdentity, signed_in_player_state};

#[spacetimedb::table(accessor = region_popuplation_loop_timer, scheduled(region_popuplation_agent_loop, at = scheduled_at))]
pub struct RegionPopulationLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn init(ctx: &ReducerContext) {
    ctx.db
        .region_popuplation_loop_timer()
        .try_insert(RegionPopulationLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(5000).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn region_popuplation_agent_loop(ctx: &ReducerContext, _timer: RegionPopulationLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to region_popuplation agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let signed_in_players = ctx.db.signed_in_player_state().count() as u32;
    let players_in_queue = ctx.db.player_queue_state().count() as u32;
    let region = ctx.db.world_region_state().id().find(0).unwrap();
    let count = RegionPopulationInfo {
        region_id: region.region_index,
        signed_in_players,
        players_in_queue,
    };
    RegionPopulationInfo::update_shared(ctx, count, crate::inter_module::InterModuleDestination::Global);
}
