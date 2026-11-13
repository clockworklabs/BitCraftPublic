use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::game::entities::location::MobileEntityState;
use crate::game::game_state::game_state_filters;
use crate::game::handlers::player::sleep;
use crate::messages::action_request::ServerTeleportReason;
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::{PlayerActionState, PlayerHousingState, PlayerState};
use crate::messages::events::{player_teleport_event, server_teleport_event, PlayerTeleportEvent, ServerTeleportEvent};
use crate::messages::util::OffsetCoordinatesFloat;
use crate::{mobile_entity_state, mounting_state, ThreatState};

#[spacetimedb::table(accessor = teleport_player_timer, scheduled(server_teleport_player, at = scheduled_at))]
pub struct TeleportPlayerTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub location: OffsetCoordinatesFloat,
    pub player_entity_id: u64,
    pub reason: ServerTeleportReason,
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn server_teleport_player(ctx: &ReducerContext, timer: TeleportPlayerTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    reduce(ctx, timer.player_entity_id, timer.location)?;
    ctx.db.player_teleport_event().insert(PlayerTeleportEvent {
        actor_id: timer.player_entity_id,
    });
    ctx.db.server_teleport_event().insert(ServerTeleportEvent {
        player_entity_id: timer.player_entity_id,
        reason: timer.reason,
    });
    Ok(())
}

pub fn reduce(ctx: &ReducerContext, player_entity_id: u64, new_coord: OffsetCoordinatesFloat) -> Result<(), String> {
    // end all combat sessions involving this player
    ThreatState::clear_all(ctx, player_entity_id);

    // clear anyone targetting the dead player
    game_state_filters::untarget(ctx, player_entity_id);

    // remove deployable when teleporting (from death or teleport command)
    if ctx.db.mounting_state().entity_id().find(player_entity_id).is_some() {
        ctx.db.mounting_state().entity_id().delete(player_entity_id);
    }

    let previous_location = ctx
        .db
        .mobile_entity_state()
        .entity_id()
        .find(player_entity_id)
        .unwrap()
        .coordinates_float();

    // Player Housing
    PlayerHousingState::update_is_empty_flag(ctx, previous_location.dimension);

    let new_coord = MobileEntityState::for_location(player_entity_id, new_coord, ctx.timestamp);

    // update movement data and discoveries
    PlayerState::move_player_and_explore(
        ctx,
        player_entity_id,
        &previous_location,
        &new_coord.coordinates_float(),
        0.0,
        false,
        None,
    )?;

    let new_chunk_index = new_coord.chunk_index;

    // force the teleport
    ctx.db.mobile_entity_state().entity_id().update(new_coord);
    PlayerActionState::update_chunk_index_on_all_layers(ctx, player_entity_id, new_chunk_index);

    // Sleep if there's a sleep building on the end-point. Ignore the error if there's no building.
    let _ = sleep::reduce(ctx, player_entity_id);

    Ok(())
}
