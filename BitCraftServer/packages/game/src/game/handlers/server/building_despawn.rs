use spacetimedb::ReducerContext;

use crate::{game::reducer_helpers::building_helpers, messages::authentication::ServerIdentity};

#[spacetimedb::table(accessor = building_despawn_timer, scheduled(building_despawn, at = scheduled_at))]
pub struct BuildingDespawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
}

#[spacetimedb::reducer]
fn building_despawn(ctx: &ReducerContext, timer: BuildingDespawnTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    building_helpers::delete_building(ctx, 0, timer.entity_id, None, false, true);

    Ok(())
}
