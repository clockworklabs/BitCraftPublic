use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::{
    building_state, elevator_desc,
    game::{
        entities::buff,
        game_state::{self, game_state_filters},
        reducer_helpers::{deployable_helpers, player_action_helpers, timer_helpers::now_plus_secs_f32},
        terrain_chunk::TerrainChunkCache,
    },
    hex_direction::HexDirection,
    location_state,
    messages::{authentication::ServerIdentity, components::HealthState, game_util::ActiveBuff},
    mobile_entity_state, unwrap_or_err, BuffCategory, BuffDesc, LocationState, MobileEntityState, PlayerActionState, PlayerActionType,
    PlayerState, PlayerTimestampState, SmallHexTile, ThreatState,
};

#[spacetimedb::table(accessor = player_use_elevator_timer, public, scheduled(player_elevator_arrive, at = scheduled_at))]
pub struct PlayerUseElevatorTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    #[unique]
    pub player_entity_id: u64,
    #[unique]
    pub origin_platform_entity_id: u64,
    #[unique]
    pub destination_platform_entity_id: u64,
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_use_elevator(ctx: &ReducerContext, platform_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if ActiveBuff::has_active_buff_of_category(ctx, actor_id, BuffCategory::ElevatorSickness) {
        return Err("Can't use an elevator right now".into());
    }

    let player_action_state = unwrap_or_err!(
        PlayerActionState::get_state(ctx, &actor_id, &PlayerActionType::UseElevator.get_layer(ctx)),
        "Invalid player action state"
    );
    if player_action_state.action_type == PlayerActionType::UseElevator {
        return Err("Already using an elevator".into());
    }

    let building = unwrap_or_err!(ctx.db.building_state().entity_id().find(platform_entity_id), "Unknown building");
    let elevator_desc = unwrap_or_err!(
        ctx.db.elevator_desc().building_id().find(building.building_description_id),
        "Building is not an elevator"
    );

    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);
    if building.distance_to(ctx, &player_coord) > 2 {
        return Err("Too far".into());
    }

    let origin_platform_location =
        unwrap_or_err!(ctx.db.location_state().entity_id().find(platform_entity_id), "Unknown Location").coordinates();
    let destination_platform_location = origin_platform_location.neighbor(HexDirection::from(building.direction_index));
    let destination_platform_entity_id = get_destination_platform_entity_id(ctx, destination_platform_location)?;

    let elevation_delta = get_elevation_delta(ctx, origin_platform_location, destination_platform_location)?;
    let traverse_duration = elevation_delta.abs() as f32 / elevator_desc.speed;

    ThreatState::clear_all(ctx, actor_id);
    game_state_filters::untarget(ctx, actor_id);
    deployable_helpers::dismount_deployable(ctx, actor_id, false);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::UseElevator,
        Some(destination_platform_entity_id),
        None,
        Duration::from_secs_f32(traverse_duration),
        Ok(()),
        game_state::unix_ms(ctx.timestamp),
    )?;

    if ctx
        .db
        .player_use_elevator_timer()
        .origin_platform_entity_id()
        .find(platform_entity_id)
        .is_some()
        || ctx
            .db
            .player_use_elevator_timer()
            .destination_platform_entity_id()
            .find(platform_entity_id)
            .is_some()
    {
        return Err("This elevator is already in use".into());
    }

    match ctx.db.player_use_elevator_timer().try_insert(PlayerUseElevatorTimer {
        scheduled_id: 0,
        scheduled_at: now_plus_secs_f32(traverse_duration, ctx.timestamp),
        player_entity_id: actor_id,
        origin_platform_entity_id: platform_entity_id,
        destination_platform_entity_id,
    }) {
        Err(_) => Err("Failed to insert PlayerUseElevatorTimer".into()),
        Ok(_) => Ok(()),
    }
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_elevator_arrive(ctx: &ReducerContext, timer: PlayerUseElevatorTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    player_action_helpers::schedule_clear_player_action(
        timer.player_entity_id,
        PlayerActionType::UseElevator.get_layer(ctx),
        player_elevator_arrive_internal(ctx, &timer),
    )
}

fn player_elevator_arrive_internal(ctx: &ReducerContext, timer: &PlayerUseElevatorTimer) -> Result<(), String> {
    let mobile_entity = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(timer.player_entity_id),
        "Unknown player location"
    );
    let destination_platform = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(timer.destination_platform_entity_id),
        "Unknown platform location"
    );

    PlayerState::move_player_and_explore(
        ctx,
        timer.player_entity_id,
        &mobile_entity.coordinates_float(),
        &destination_platform.coordinates().into(),
        0.0,
        false,
        None,
    )?;

    let mobile_entity = MobileEntityState::for_location(
        timer.player_entity_id,
        destination_platform.offset_coordinates().into(),
        ctx.timestamp,
    );
    ctx.db.mobile_entity_state().entity_id().update(mobile_entity);

    if let Some(elevator_sickness_debuff) = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::ElevatorSickness) {
        let _ = buff::activate(
            ctx,
            timer.player_entity_id,
            elevator_sickness_debuff.id,
            Some(elevator_sickness_debuff.duration),
            None,
        );
    }

    Ok(())
}

pub fn get_destination_platform_entity_id(ctx: &ReducerContext, destination_platform: SmallHexTile) -> Result<u64, String> {
    for location in LocationState::select_all(ctx, &destination_platform) {
        if let Some(building) = ctx.db.building_state().entity_id().find(location.entity_id) {
            if ctx
                .db
                .elevator_desc()
                .building_id()
                .find(building.building_description_id)
                .is_some()
            {
                return Ok(building.entity_id);
            }
        }
    }

    Err("Elevator has no second platform".into())
}

pub fn get_elevation_delta(
    ctx: &ReducerContext,
    platform_location: SmallHexTile,
    destination_platform: SmallHexTile,
) -> Result<i16, String> {
    let mut cache = TerrainChunkCache::empty();
    let terrain_cell = unwrap_or_err!(
        cache.get_terrain_cell(ctx, &platform_location.parent_large_tile()),
        "Invalid platform location"
    );
    let adjacent_terrain_cell = unwrap_or_err!(
        cache.get_terrain_cell(ctx, &destination_platform.parent_large_tile()),
        "Invalid platform location"
    );

    Ok(terrain_cell.elevation - adjacent_terrain_cell.elevation)
}
