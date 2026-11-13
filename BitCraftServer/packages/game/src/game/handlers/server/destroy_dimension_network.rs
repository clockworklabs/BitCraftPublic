use spacetimedb::ReducerContext;

use crate::{game::reducer_helpers::interior_helpers, messages::authentication::ServerIdentity, OffsetCoordinatesFloat};

#[spacetimedb::table(accessor = destroy_dimension_network_timer, scheduled(destroy_dimension_network, at = scheduled_at))]
pub struct DestroyDimensionNetworkTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub player_teleport_location: OffsetCoordinatesFloat,
    pub dimension_network_entity_id: u64,
}

#[spacetimedb::reducer]
fn destroy_dimension_network(ctx: &ReducerContext, timer: DestroyDimensionNetworkTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let teleport_oc = timer.player_teleport_location;
    interior_helpers::delete_dimension_network(ctx, timer.dimension_network_entity_id, teleport_oc);

    Ok(())
}
