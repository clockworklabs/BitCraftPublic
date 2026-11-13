use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

use crate::messages::events::*;
use crate::{
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    messages::{action_request::*, components::*},
    parameters_desc, params, unwrap_or_err,
};

use super::sleep;

pub fn event_delay(ctx: &ReducerContext, _actor_id: u64, _request: &PlayerTeleportWaystoneRequest) -> Duration {
    Duration::from_secs_f32(params!(ctx).teleport_channel_time_waystone)
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_teleport_waystone_start(ctx: &ReducerContext, request: PlayerTeleportWaystoneRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let delay = event_delay(ctx, actor_id, &request);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Teleport,
        None,
        None,
        delay,
        reduce(ctx, actor_id, request.entity_id_from, request.entity_id_to, true),
        game_state::unix_ms(ctx.timestamp),
    );
    if result.is_ok() {
        ctx.db
            .player_teleport_waystone_start_event()
            .insert(PlayerTeleportWaystoneStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_teleport_waystone(ctx: &ReducerContext, request: PlayerTeleportWaystoneRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let entity_id_from = request.entity_id_from;
    let entity_id_to = request.entity_id_to;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    //get building location
    let location_state_to = unwrap_or_err!(ctx.db.location_state().entity_id().find(entity_id_to), "Waystone does not exist.");
    let coords = location_state_to.coordinates();
    let teleport_location_tile = coords;

    let result = if sleep::can_sleep(ctx, teleport_location_tile).is_ok() {
        let r = reduce(ctx, actor_id, entity_id_from, entity_id_to, false);
        if r.is_err() {
            player_action_helpers::schedule_clear_player_action(actor_id, PlayerActionType::Teleport.get_layer(ctx), r.clone())
        } else {
            r
        }
    } else {
        player_action_helpers::schedule_clear_player_action(
            actor_id,
            PlayerActionType::Teleport.get_layer(ctx),
            reduce(ctx, actor_id, entity_id_from, entity_id_to, false),
        )
    };

    if result.is_ok() {
        ctx.db.player_teleport_event().insert(PlayerTeleportEvent { actor_id });
    }
    result
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, entity_id_from: u64, entity_id_to: u64, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Teleport, None)?;
    }

    // This transaction happens in old chunk. Don't update player position to make sure it gets sent to players in old chunk
    // todo: handle teleport while in a deployable carrying passengers

    if ThreatState::in_combat(ctx, actor_id) {
        return Err("Can't teleport while in combat!".into());
    }

    if InventoryState::get_player_cargo_id(ctx, actor_id) != 0 {
        return Err("Cannot teleport while carrying a cargo".into());
    }

    if dry_run {
        return Ok(());
    }

    // Sleep if there's a sleep building on the end-point. Ignore the error if there's no building.
    let _ = sleep::reduce(ctx, actor_id);

    game_state_filters::teleport_waystone(ctx, actor_id, entity_id_from, entity_id_to, dry_run)?;

    Ok(())
}
