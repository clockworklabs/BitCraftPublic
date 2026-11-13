use crate::game::game_state::game_state_filters;
use crate::game::handlers::authentication::has_role_no_dev;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::authentication::Role;
use crate::{
    game::{coordinates::*, entities::location::MobileEntityState, game_state, reducer_helpers::move_validation_helpers},
    messages::{action_request::PlayerMoveRequest, components::*, static_data::*},
    unwrap_or_err,
};
use crate::messages::events::*;
use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_move(ctx: &ReducerContext, request: PlayerMoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, false)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if ctx.db.mounting_state().entity_id().find(actor_id).is_some() {
        return Err("Can't walk while in a deployable.".into());
    }

    let player_stats = ctx.db.character_stats_state().entity_id().find(actor_id).unwrap();
    let prev_mobile_entity = ctx.db.mobile_entity_state().entity_id().find(actor_id).unwrap();

    let prev_origin = prev_mobile_entity.coordinates_float();
    let target_coordinates: FloatHexTile = unwrap_or_err!(request.destination, "Expected destination in move request").into();
    let source_coordinates: FloatHexTile = unwrap_or_err!(request.origin, "Expected origin in move request").into();

    let paving = if request.move_type <= 2 {
        PavedTileState::get_at_location(ctx, &prev_origin.parent_small_tile())
            .map(|t| ctx.db.paving_tile_desc().id().find(t.tile_type_id).unwrap())
    } else {
        None
    };

    let source_large = source_coordinates.parent_large_tile();
    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_chunk = unwrap_or_err!(
        terrain_cache.get_from_chunk_coordinates(ctx, source_large.chunk_coordinates()),
        "You can't go here!"
    );

    //Speed on previous move segment
    let prev_chunk_index = terrain_chunk.get_index(prev_origin.parent_large_tile());
    let cur_chunk_index = terrain_chunk.get_index(source_coordinates.parent_large_tile());
    let water_body_type = terrain_chunk
        .get_water_body_type_index(prev_chunk_index)
        .unwrap_or(SurfaceType::Ground as u8);
    let water_depth = terrain_chunk.get_water_depth_index(prev_chunk_index);
    let speed = game_state_filters::get_speed_on_water_type(
        &ctx.db.parameters_player_move_desc().version().find(0).unwrap().default_speed,
        water_body_type,
        Some(water_depth),
        true,
    );

    //Make sure players aren't phasing through cliffs
    if prev_chunk_index != cur_chunk_index {
        let terrain_start = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &prev_origin.parent_large_tile()),
            "Invalid coordinates"
        );
        let terrain_target = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &source_coordinates.parent_large_tile()),
            "Invalid coordinates"
        );
        let elevation_diff = terrain_target.player_surface_elevation() - terrain_start.player_surface_elevation();
        if elevation_diff.abs() > 6 {
            return Err("~Origin elevation mismatch".into());
        }
    }

    if !has_role_no_dev(ctx, &ctx.sender(), Role::Gm) {
        move_validation_helpers::validate_move_timestamp(prev_mobile_entity.timestamp, request.timestamp, ctx.timestamp)?;
        move_validation_helpers::validate_move_basic(ctx, &prev_origin, &source_coordinates, &target_coordinates, request.duration)?;
        validate_move(
            ctx,
            actor_id,
            &player_stats,
            speed,
            &prev_mobile_entity,
            &request,
            source_coordinates,
            target_coordinates,
            &paving,
        )?;
    }

    PlayerState::move_player_and_explore(
        ctx,
        actor_id,
        &source_coordinates,
        &target_coordinates,
        0.0,
        request.is_rp_walk,
        Some(request.timestamp),
    )?;

    PlayerActionState::success(
        ctx,
        actor_id,
        if source_coordinates == target_coordinates {
            PlayerActionType::None
        } else {
            PlayerActionType::PlayerMove
        },
        PlayerActionType::PlayerMove.get_layer(ctx),
        (request.duration * 1000.0) as u64,
        None,
        None,
        request.timestamp,
    );

    ctx.db.player_move_event().insert(PlayerMoveEvent {
        actor_id,
        request,
    });

    Ok(())
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn validate_move(
    ctx: &ReducerContext,
    actor_id: u64,
    player_stats: &CharacterStatsState,
    speed: f32,
    prev_mobile_entity: &MobileEntityState,
    request: &PlayerMoveRequest,
    source_coordinates: FloatHexTile,
    _target_coordinates: FloatHexTile,
    paving: &Option<PavingTileDesc>,
) -> Result<(), String> {
    let prev_origin = prev_mobile_entity.coordinates_float();

    // if source_coordinates.x != target_coordinates.x || source_coordinates.z != target_coordinates.z {
    if source_coordinates.x != prev_origin.x || source_coordinates.z != prev_origin.z {
        let mut prev_speed = speed * player_stats.get(CharacterStatType::MovementMultiplier);
        if request.move_type > 2 {
            prev_speed *= 2.0; //Transitions are above the law
        }

        if let Some(paving) = paving {
            prev_speed = paving.apply_stat_to_value_unclamped(prev_speed, crate::CharacterStatType::MovementMultiplier);
        }

        //let (cur_position, cur_distance) = prev_mobile_entity.cur_coord_and_distance_traveled(prev_speed);

        let timestamp_diff_ms = request.timestamp - prev_mobile_entity.timestamp;
        if let Err(error) =
            move_validation_helpers::validate_move_origin(&prev_origin, &source_coordinates, timestamp_diff_ms, prev_speed, actor_id)
        {
            //Can return Err or Ok
            return move_validation_helpers::move_validation_strike(
                ctx,
                actor_id,
                actor_id,
                prev_origin,
                format!("Player {actor_id}"),
                error,
            );
        }

        //DAB Note
        // TODO: enable this at some point
        //let par = ctx.db.parameters_desc().version().find(&0).unwrap();
        //if let Err(error) = reducer_helpers::validate_move(
        //    &prev_mobile_entity,
        //    &prev_origin,
        //    &source_coordinates,
        //    &target_coordinates,
        //    par.player_climb_height as i32,
        //    par.player_swim_height as i32,
        //    MovementType::Amphibious,
        //    prev_speed,
        //    new_speed,
        //    request.duration,
        //    actor_id,
        //) {
        //    //return fail_validation(error, actor_id, cur_position);
        //    return fail_validation(error, actor_id, prev_origin);
        //}
    }

    Ok(())
}
