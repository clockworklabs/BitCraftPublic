use spacetimedb::{log, ReducerContext, Table, Timestamp};

use crate::{
    location_state,
    messages::{
        components::{active_connection_state, signed_in_player_state, LocationState, MobileEntityState},
        generic::globals,
        global::user_region_state,
    },
    mobile_entity_state, user_state,
};

use super::coordinates::*;

pub mod game_state_filters;

pub fn unix(now: Timestamp) -> i32 {
    return now.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_secs() as i32;
}

pub fn unix_ms(now: Timestamp) -> u64 {
    return now.duration_since(Timestamp::UNIX_EPOCH).unwrap().as_millis() as u64;
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
    // Only the newest connection may act; None means a non-client caller.
    if let (Some(connection_id), Some(active)) = (ctx.connection_id, ctx.db.active_connection_state().entity_id().find(&entity_id)) {
        if connection_id != active.connection_id {
            return Err("Stale connection".into());
        }
    }
    return Ok(());
}

pub fn region_index_from_entity_id(entity_id: u64) -> u8 {
    return (entity_id >> 56) as u8;
}

pub fn player_region(ctx: &ReducerContext, entity_id: u64) -> Result<u8, String> {
    let user_state = ctx.db.user_state().entity_id().find(entity_id).expect("User doesn't exist");
    let region_state = ctx
        .db
        .user_region_state()
        .identity()
        .find(user_state.identity)
        .expect("Player region not found");
    return Ok(region_state.region_id);
}
