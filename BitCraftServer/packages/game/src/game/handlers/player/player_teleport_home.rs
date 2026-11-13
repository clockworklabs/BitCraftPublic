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
    messages::{action_request::*, components::*, static_data::parameters_desc},
    params, player_state, unwrap_or_err, SmallHexTile,
};

use super::sleep;

pub fn event_delay(ctx: &ReducerContext, _actor_id: u64, _request: &PlayerTeleportHomeRequest) -> Duration {
    Duration::from_secs_f32(params!(ctx).teleport_channel_time_home)
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_teleport_home_start(ctx: &ReducerContext, request: PlayerTeleportHomeRequest) -> Result<(), String> {
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
        reduce(ctx, actor_id, true),
        game_state::unix_ms(ctx.timestamp),
    );
    if result.is_ok() {
        ctx.db
            .player_teleport_home_start_event()
            .insert(PlayerTeleportHomeStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_teleport_home(ctx: &ReducerContext, _request: PlayerTeleportHomeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let player = ctx.db.player_state().entity_id().find(actor_id).unwrap();
    let teleport_location = player.teleport_location;
    let teleport_location_tile = SmallHexTile::from(teleport_location.location);

    let result = if sleep::can_sleep(ctx, teleport_location_tile).is_ok() {
        let r = reduce(ctx, actor_id, false);
        if r.is_err() {
            player_action_helpers::schedule_clear_player_action(actor_id, PlayerActionType::Teleport.get_layer(ctx), r.clone())
        } else {
            r
        }
    } else {
        player_action_helpers::schedule_clear_player_action(
            actor_id,
            PlayerActionType::Teleport.get_layer(ctx),
            reduce(ctx, actor_id, false),
        )
    };

    if result.is_ok() {
        ctx.db.player_teleport_event().insert(PlayerTeleportEvent { actor_id });
    }
    result
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Teleport, None)?;
    } else {
        let action_state = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &actor_id, &PlayerActionLayer::Base),
            "Player has no ActionState"
        );
        if action_state.action_type != PlayerActionType::None
            && action_state.action_type != PlayerActionType::PlayerMove
            && action_state.action_type != PlayerActionType::DeployableMove
        {
            // Prevents an exploit where you can climb down a high cliff, click teleport, cancel, then move to skip the entire climb sequence.
            return Err("Cannot teleport back home now".into());
        }
    }

    // This transaction happens in old chunk. Don't update player position to make sure it gets sent to players in old chunk
    // todo: handle teleport while in a deployable carrying passengers

    if ThreatState::in_combat(ctx, actor_id) {
        return Err("Can't teleport while in combat!".into());
    }

    if InventoryState::get_player_cargo_id(ctx, actor_id) != 0 {
        return Err("Cannot teleport back home while carrying a cargo".into());
    }

    if dry_run {
        return Ok(());
    }

    game_state_filters::teleport_home(ctx, actor_id, false)?;

    // Sleep if there's a sleep building on the end-point. Ignore the error if there's no building.
    let _ = sleep::reduce(ctx, actor_id);

    Ok(())
}
