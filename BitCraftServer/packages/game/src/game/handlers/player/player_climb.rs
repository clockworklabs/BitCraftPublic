use bitcraft_macro::feature_gate;
use std::time::Duration;

use crate::climb_requirement_desc;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::{
    game::{coordinates::*, entities::location::MobileEntityState, game_state},
    messages::events::*,
    messages::{action_request::PlayerClimbRequest, components::*, static_data::ClimbRequirementDesc},
    unwrap_or_err,
};
use spacetimedb::{ReducerContext, Table};

const MAX_DISTANCE: f32 = 15.0;

fn event_delay(ctx: &ReducerContext, terrain_cache: &mut TerrainChunkCache, request: &PlayerClimbRequest) -> Result<Duration, String> {
    let start_coord = FloatHexTile::from(request.origin);
    let end_coord = FloatHexTile::from(request.destination);
    let start_cell = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &LargeHexTile::from(start_coord)),
        "Invalid climb coordinates"
    );
    let end_cell = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &LargeHexTile::from(end_coord)),
        "Invalid climb coordinates"
    );
    let start_elevation = start_cell.player_surface_elevation();
    let end_elevation = end_cell.player_surface_elevation();
    let elevation_diff = end_elevation - start_elevation;

    let desc = unwrap_or_err!(get_climb_descriptions(elevation_diff), "Invalid climb request");
    let time = desc.get_climb_time(elevation_diff);

    Ok(Duration::from_secs_f32(time))
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_climb_start(ctx: &ReducerContext, request: PlayerClimbRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut terrain_cache = TerrainChunkCache::empty();
    let target = Some(FloatHexTile::from(request.destination).parent_large_tile().hashcode_long() as u64);
    let delay = event_delay(ctx, &mut terrain_cache, &request);
    let result = match delay {
        Err(error) => player_action_helpers::fail_action(actor_id, PlayerActionType::Climb.get_layer(ctx), error),
        Ok(delay) => player_action_helpers::start_action(
            ctx,
            actor_id,
            PlayerActionType::Climb,
            target,
            None,
            delay,
            reduce(ctx, &mut terrain_cache, actor_id, &request, true),
            request.timestamp,
        ),
    };
    if result.is_ok() {
        ctx.db
            .player_climb_start_event()
            .insert(PlayerClimbStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_climb(ctx: &ReducerContext, request: PlayerClimbRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::Climb.get_layer(ctx),
        reduce(ctx, &mut terrain_cache, actor_id, &request, false),
    )
}

fn reduce(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    request: &PlayerClimbRequest,
    is_start_reducer: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if ctx.db.mounting_state().entity_id().find(actor_id).is_some() {
        return Err("Can't walk while in a deployable.".into());
    }

    let start_coordinates: FloatHexTile = request.origin.into();
    let target_coordinates: FloatHexTile = request.destination.into();

    if start_coordinates.distance_to(target_coordinates) > MAX_DISTANCE {
        return Err("Can't climb that far.".into());
    }

    let mobile = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Invalid request");
    if mobile.coordinates_float().distance_to(start_coordinates) > 2f32 {
        return Err("Invalid origin.".into());
    }

    let start_large_tile = LargeHexTile::from(start_coordinates);
    let target_large_tile = LargeHexTile::from(target_coordinates);

    if !is_start_reducer {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(
            ctx,
            actor_id,
            PlayerActionType::Climb,
            Some(target_large_tile.hashcode_long() as u64),
        )?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Climb, request.timestamp)?;
    }

    if start_large_tile.direction(&target_large_tile).is_none() {
        return Err("Invalid climb, target is too far.".into());
    }

    let terrain_start = unwrap_or_err!(terrain_cache.get_terrain_cell(ctx, &start_large_tile), "Invalid coordinates");
    let terrain_target = unwrap_or_err!(terrain_cache.get_terrain_cell(ctx, &target_large_tile), "Invalid coordinates");

    let elevation_diff = terrain_target.player_surface_elevation() - terrain_start.player_surface_elevation();
    let climb_desc = unwrap_or_err!(get_climb_descriptions(elevation_diff), "Not a climb");

    if is_start_reducer {
        if request.climb_type != climb_desc.climb_type
            //ClimbDown can play instead of Down1 and Down2
            && !(request.climb_type == 140 && (climb_desc.climb_type == 110 || climb_desc.climb_type == 120))
        {
            //Only check this on start to avoid erroring climbs onto cliffs that get terraformed halfway through the climb
            return Err("Invalid climb type".into());
        }

        let climb_requirement_desc = unwrap_or_err!(get_climb_requirement_desc(ctx, elevation_diff), "Not a climb");
        let stamina_cost = climb_requirement_desc.stamina_cost as f32;
        let stamina_state = ctx.db.stamina_state().entity_id().find(actor_id).unwrap();
        if stamina_state.stamina < stamina_cost {
            return Err("Not enough stamina".into());
        }

        let climbing_proficiency = CharacterStatsState::get_entity_stat(ctx, actor_id, crate::CharacterStatType::ClimbProficiency);
        if climbing_proficiency < climb_requirement_desc.min_climb_proficiency {
            return Err("You don't have the required level of climb proficiency".into());
        }

        //Start move in start reducer
        PlayerState::move_player_and_explore(
            ctx,
            actor_id,
            &start_coordinates,
            &target_coordinates,
            -stamina_cost,
            false,
            Some(request.timestamp),
        )?;
    } else {
        let target_offset_coordinates = OffsetCoordinatesFloat::from(target_coordinates);
        let mut mes = MobileEntityState::for_location(actor_id, target_offset_coordinates, ctx.timestamp);
        mes.timestamp = request.timestamp;
        ctx.db.mobile_entity_state().entity_id().update(mes);
    }

    Ok(())
}

fn get_climb_requirement_desc(ctx: &ReducerContext, elevation_diff: i16) -> Option<ClimbRequirementDesc> {
    return ctx
        .db
        .climb_requirement_desc()
        .iter().find(|a| elevation_diff >= a.min_elevation && elevation_diff <= a.max_elevation);
}

#[derive(Clone)]
struct ClimbAnimationDescription {
    pub climb_type: i32,
    pub min_elevation: i16,
    pub max_elevation: i16,
    pub climb_time_flat: f32,
    pub climb_tile_elevation_offset: i16,
    pub climb_time_per_elevation_unit: f32,
}

//i32::min isn't const for some reason :/
const fn min_const(a: i16, b: i16) -> i16 {
    if a < b { a } else { b }
}

//i32::max isn't const for some reason :/
const fn max_const(a: i16, b: i16) -> i16 {
    if a > b { a } else { b }
}

impl ClimbAnimationDescription {
    pub const fn new_const_time(climb_type: i32, elevation_range_1: i16, elevation_range_2: i16, climb_time_flat: f32) -> Self {
        ClimbAnimationDescription {
            climb_type, //Hoist1
            min_elevation: min_const(elevation_range_1, elevation_range_2),
            max_elevation: max_const(elevation_range_1, elevation_range_2),
            climb_time_flat,
            climb_time_per_elevation_unit: 0.0,
            climb_tile_elevation_offset: 0,
        }
    }

    pub const fn new_var_time(
        climb_type: i32,
        elevation_range_1: i16,
        elevation_range_2: i16,
        climb_time_flat: f32,
        climb_tile_elevation_offset: i16,
        climb_time_per_elevation_unit: f32,
    ) -> Self {
        ClimbAnimationDescription {
            climb_type, //Hoist1
            min_elevation: min_const(elevation_range_1, elevation_range_2),
            max_elevation: max_const(elevation_range_1, elevation_range_2),
            climb_time_flat,
            climb_tile_elevation_offset,
            climb_time_per_elevation_unit,
        }
    }

    pub fn get_climb_time(&self, elevation_diff: i16) -> f32 {
        let adjusted_elevation = elevation_diff.abs() - self.climb_tile_elevation_offset.abs();
        self.climb_time_flat + self.climb_time_per_elevation_unit * adjusted_elevation as f32
    }
}

//These values are duplicated in ClimbController.cs
const CLIMB_DESCRIPTIONS: [ClimbAnimationDescription; 7] = [
    //Up
    ClimbAnimationDescription::new_const_time(
        //Hoist1
        10,
        5,
        7,
        18.0 / 24.0,
    ),
    ClimbAnimationDescription::new_const_time(
        //Hoist2
        20,
        8,
        10,
        18.0 / 24.0,
    ),
    ClimbAnimationDescription::new_var_time(
        //ClimbUp
        30,
        11,
        i16::MAX,
        1.0, // 24.0 / 24.0
        7,
        26.0 / 24.0 / 4.5,
    ),
    //Down
    ClimbAnimationDescription::new_const_time(
        //Down1
        110,
        -5,
        -7,
        18.0 / 24.0,
    ),
    ClimbAnimationDescription::new_const_time(
        //Down2
        120,
        -8,
        -10,
        18.0 / 24.0,
    ),
    ClimbAnimationDescription::new_var_time(
        //ClimbDown
        130,
        -11,
        i16::MIN,
        26.0 / 24.0,
        7,
        26.0 / 24.0 / 4.5,
    ),
    ClimbAnimationDescription::new_const_time(
        //Down1Swim
        140,
        -5,
        -10,
        21.0 / 24.0 / 0.9,
    ),
];

fn get_climb_descriptions(elevation_diff: i16) -> Option<ClimbAnimationDescription> {
    CLIMB_DESCRIPTIONS
        .iter().find(|a| elevation_diff >= a.min_elevation && elevation_diff <= a.max_elevation)
        .cloned()
}
