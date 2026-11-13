use crate::{
    game::entities::location::MobileEntityState,
    messages::{authentication::ServerIdentity, components::*},
    OffsetCoordinatesFloat,
};
use spacetimedb::ReducerContext;

#[spacetimedb::table(accessor = reset_mobile_entity_timer, scheduled(reset_mobile_entity_position, at = scheduled_at))]
pub struct ResetMobileEntityTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub owner_entity_id: u64,
    pub position: Option<OffsetCoordinatesFloat>,
    pub strike_counter_to_update: Option<MoveValidationStrikeCounterState>,
}

#[spacetimedb::reducer]
pub fn reset_mobile_entity_position(ctx: &ReducerContext, timer: ResetMobileEntityTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    ctx.db.mobile_entity_state().entity_id().update(MobileEntityState::for_location(
        timer.owner_entity_id,
        timer.position.unwrap(),
        ctx.timestamp,
    ));

    if let Some(strike_counter) = timer.strike_counter_to_update {
        ctx.db.move_validation_strike_counter_state().entity_id().update(strike_counter);
    }

    PlayerActionState::clear_by_entity_id(ctx, timer.owner_entity_id)
}
