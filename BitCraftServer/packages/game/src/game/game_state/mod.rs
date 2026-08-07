use spacetimedb::{log, ReducerContext, Table, Timestamp};

use crate::{
    location_state,
    messages::{
        components::{active_connection_state, signed_in_player_state, LocationState, MobileEntityState},
        generic::globals,
    },
    mobile_entity_state, user_state,
};

use super::coordinates::*;

pub mod game_state_filters;
pub mod wind_system;

pub fn unix(now: Timestamp) -> i32 {
    return now.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_secs() as i32;
}

pub fn unix_ms(now: Timestamp) -> u64 {
    return now.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_millis() as u64;
}

pub fn days_since_unix_epoch(now: Timestamp) -> i32 {
    return (now.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_secs() / 86_400) as i32;
}

pub fn insert_location(ctx: &ReducerContext, entity_id: u64, offset_coordinates: OffsetCoordinatesSmall) {
    if ctx
        .db
        .location_state()
        .try_insert(LocationState::new(entity_id, offset_coordinates))
        .is_err()
    {
        log::error!("Failed to insert location");
    }
}

pub fn insert_location_float(ctx: &ReducerContext, entity_id: u64, coordinates: OffsetCoordinatesFloat) {
    if ctx
        .db
        .mobile_entity_state()
        .try_insert(MobileEntityState::for_location(entity_id, coordinates, ctx.timestamp))
        .is_err()
    {
        log::error!("Failed to insert mobile entity state");
    }
}

pub fn create_entity(ctx: &ReducerContext) -> u64 {
    let mut globals = ctx.db.globals().version().find(&0).unwrap();
    globals.entity_pk_counter += 1;
    let pk = globals.entity_pk_counter;
    let pk = pk | ((globals.region_index as u64) << 56);
    ctx.db.globals().version().update(globals);

    pk
}

pub fn create_dimension(ctx: &ReducerContext) -> u32 {
    let mut globals = ctx.db.globals().version().find(&0).unwrap();
    globals.dimension_counter += 1;
    let pk = globals.dimension_counter;
    ctx.db.globals().version().update(globals);

    pk
}

pub fn actor_id(ctx: &ReducerContext, must_be_signed_in: bool) -> Result<u64, String> {
    match ctx.db.user_state().identity().find(&ctx.sender) {
        Some(user) => {
            if must_be_signed_in {
                ensure_signed_in(ctx, user.entity_id)?;
            }
            Ok(user.entity_id)
        }
        None => Err("Invalid sender".into()),
    }
}

pub fn ensure_signed_in(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if ctx.db.signed_in_player_state().entity_id().find(&entity_id).is_none() {
        return Err("Not signed in".into());
    }
    // Only the newest connection may act; None means a non-player caller.
    if let (Some(connection_id), Some(active)) = (ctx.connection_id, ctx.db.active_connection_state().entity_id().find(&entity_id)) {
        if connection_id != active.connection_id {
            return Err("Stale connection".into());
        }
    }
    return Ok(());
}
