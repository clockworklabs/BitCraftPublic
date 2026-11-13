use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::FloatHexTile;
use crate::game::game_state::game_state_filters::coordinates_float;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::events::*;
use crate::{deployable_desc, emote_desc};
use crate::{
    game::{game_state, reducer_helpers::player_action_helpers},
    messages::{action_request::PlayerEmoteRequest, components::*, static_data::EmoteDesc},
    unwrap_or_err,
};

pub fn event_delay(emote: &EmoteDesc) -> Duration {
    if emote.duration <= 0.0 {
        Duration::MAX //Looping emotes don't have end time
    } else {
        Duration::from_secs_f32(emote.duration)
    }
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn emote_start(ctx: &ReducerContext, request: PlayerEmoteRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let emote = unwrap_or_err!(ctx.db.emote_desc().id().find(request.emote_id), "Invalid emote");
    let delay = event_delay(&emote);
    let action_type = if emote.allow_while_moving {
        PlayerActionType::MobileEmote
    } else {
        PlayerActionType::StationaryEmote
    };

    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        action_type,
        None,
        None,
        delay,
        reduce(ctx, actor_id, request.emote_id, true),
        game_state::unix_ms(ctx.timestamp),
    );
    if result.is_ok() {
        ctx.db.emote_start_event().insert(EmoteStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn emote(ctx: &ReducerContext, request: PlayerEmoteRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let emote = unwrap_or_err!(ctx.db.emote_desc().id().find(request.emote_id), "Invalid emote");
    let action_type = if emote.allow_while_moving {
        PlayerActionType::MobileEmote
    } else {
        PlayerActionType::StationaryEmote
    };

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        action_type.get_layer(ctx),
        reduce(ctx, actor_id, request.emote_id, false),
    )
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, emote_id: i32, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    //check if player is climbing
    let player_action_state = unwrap_or_err!(
        PlayerActionState::get_state(ctx, &actor_id, &PlayerActionType::Climb.get_layer(ctx)),
        "Missing player action state"
    );
    if player_action_state.action_type == PlayerActionType::Climb {
        return Err("Cannot emote while climbing".into());
    }

    let emote = unwrap_or_err!(ctx.db.emote_desc().id().find(emote_id), "Invalid emote");

    let vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(actor_id), "Player missing VaultState");
    let mut has_in_vault = false;
    for collectible in vault.collectibles {
        if collectible.id == emote.enabled_by_collectible_id {
            has_in_vault = true;
            break;
        }
    }
    if !has_in_vault {
        return Err("You haven't unlocked this emote".into());
    }

    let mut is_passenger = false;
    let mut is_driver = false;

    //validate emotes that cannot be use while mounted
    if let Some(mounting_state) = ctx.db.mounting_state().entity_id().find(actor_id) {
        if !emote.allow_while_mounted {
            return Err("Cannot use this emote while mounted".into());
        }

        //validate emotes that cannot be use in certain deployables
        if let Some(deployable_state) = ctx.db.deployable_state_v2().entity_id().find(mounting_state.deployable_entity_id) {
            let deployable_desc = unwrap_or_err!(
                ctx.db.deployable_desc().id().find(deployable_state.deployable_description_id),
                "Deployable does not exist"
            );

            //validate passenger emote
            is_passenger = mounting_state.deployable_slot != 0;
            if is_passenger && !deployable_desc.allow_emote_while_passenger {
                return Err("Cannot use this emote while a passenger".into());
            }

            // Owner is always the driver
            is_driver = deployable_state.owner_id == actor_id;

            //validate driver emote
            if is_driver && !deployable_desc.allow_emote_while_driver {
                return Err("Cannot use this emote while driving".into());
            }
        }
    }

    //not in deployable, check if player is swimming
    if !is_passenger && !is_driver {
        let actor_coords: FloatHexTile = coordinates_float(ctx, actor_id);
        let mut terrain_cache = TerrainChunkCache::empty();
        let terrain_source = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &actor_coords.parent_large_tile()),
            "Invalid location"
        );

        if terrain_source.player_should_swim() {
            return Err("Cannot emote while swimming".into());
        }
    }

    if dry_run {
        return Ok(());
    }

    let action_type = if emote.allow_while_moving {
        PlayerActionType::MobileEmote
    } else {
        PlayerActionType::StationaryEmote
    };

    // Make sure target and timestamp and action fit
    PlayerActionState::validate(ctx, actor_id, action_type, None)?;

    // ¯\_(ツ)_/¯
    player_action_helpers::schedule_clear_player_action(actor_id, action_type.get_layer(ctx), Ok(()))
}
