use spacetimedb::{log, ReducerContext, Table, Timestamp};

use crate::{
    dimension_description_state,
    game::{
        coordinates::{self, ChunkCoordinates, FloatHexTile, SmallHexTile},
        dimensions,
        game_state::{self, game_state_filters},
        handlers::server::reset_mobile_entity_position::{reset_mobile_entity_timer, ResetMobileEntityTimer},
        reducer_helpers::player_action_helpers,
        unity_helpers::vector2::Vector2,
    },
    messages::components::PlayerActionType,
    move_validation_strike_counter_state, private_parameters_desc, MoveValidationStrikeCounterState, MovementType, OffsetCoordinatesFloat,
};

use crate::messages::components::MobileEntityState;

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
pub fn validate_move(
    ctx: &ReducerContext,
    prev_state: &MobileEntityState,
    prev_origin: &FloatHexTile,
    source_coordinates: &FloatHexTile,
    target_coordinates: &FloatHexTile,
    _max_elevation_land: i32,
    _max_elevation_water: i32,
    _movement_type: MovementType,
    _prev_move_speed: f32,
    _move_speed: f32,
    duration: f32,
    player_id: u64,
) -> Result<(), String> {
    const MAX_DURATION: f32 = 100.0;
    //Sanity checks
    if duration > MAX_DURATION {
        return Err("Invalid duration.".into());
    }
    if source_coordinates.dimension != target_coordinates.dimension {
        return Err("Invalid dimension.".into());
    }

    if target_coordinates.dimension != prev_state.dimension {
        return Err("Client sent wrong dimension".into());
    }

    //Can't move outside world bounds
    let target_coordinates_int: SmallHexTile = (target_coordinates).into();
    let target_coordinates_chunk: ChunkCoordinates = (&target_coordinates_int).into();
    let dimension_desc = ctx
        .db
        .dimension_description_state()
        .dimension_id()
        .find(target_coordinates.dimension)
        .unwrap();
    if (target_coordinates_chunk.x < dimension_desc.dimension_position_large_x as i32)
        | (target_coordinates_chunk.z < dimension_desc.dimension_position_large_z as i32)
        | (target_coordinates_chunk.x >= dimension_desc.dimension_position_large_x as i32 + dimension_desc.dimension_size_large_x as i32)
        | (target_coordinates_chunk.z >= dimension_desc.dimension_position_large_z as i32 + dimension_desc.dimension_size_large_z as i32)
    {
        return Err("Cannot move outside of world bounds".into());
    }

    let (prev_origin_small, prev_origin_large) = prev_origin.parent_small_and_large_tile();
    let (source_small, source_large) = source_coordinates.parent_small_and_large_tile();
    let (_target_small, target_large) = target_coordinates.parent_small_and_large_tile();

    if source_large.distance_to(target_large) > 3 {
        spacetimedb::log::warn!("Destination distance is {}", source_coordinates.distance_to(*target_coordinates));
        return Err("Invalid destination".into());
    }
    if prev_origin_large.distance_to(source_large) > 3 {
        spacetimedb::log::warn!("Origin distance is {}", source_coordinates.distance_to(*target_coordinates));
        return Err("Origin is too far".into());
    }

    //Check if received origin is too far from server origin
    //let time_delta = game_state::moment_milliseconds() - prev_state.timestamp;
    //let origin_distance = (prev_origin.to_world_position() - source_coordinates.to_world_position()).magnitude();
    //let max_distance = (time_delta as f32 * prev_move_speed / 1000.0 * 2.0).clamp(9.0, 30.0);
    //if origin_distance > max_distance {
    //    spacetimedb::log::warn!("Origin distance is {}, expected max {}", origin_distance, max_distance);
    //    return Err("Invalid origin".into());
    //}

    //if prev_origin.distance_to(source_coordinates) > (cur_distance * 1.5).max(5.0) {
    //    spacetimedb::log::warn!(
    //        "Origin distance is {}, expected {}",
    //        prev_origin.distance_to(source_coordinates),
    //        cur_distance * 1.5
    //    );
    //    return fail_validation("Invalid origin".into(), actor_id, cur_position);
    //}

    // Can't move onto "hitbox" footprints
    //if game_state_filters::has_hitbox_footprint(target_small) {
    //    log::warn!("Player {} tried to walk to {:?} which is a hitbox", player_id, target_coordinates);
    //    return Err("Can't walk through here!".into());
    //}

    if let Some(new_hitbox) = game_state_filters::get_hitbox_footprint(ctx, source_small) {
        let old_hitbox = game_state_filters::get_hitbox_footprint(ctx, prev_origin_small);
        //Only allow moving on hitboxes when you're leaving a building that was built on top of you
        if old_hitbox.is_none() || old_hitbox.unwrap().owner_entity_id != new_hitbox.owner_entity_id {
            log::warn!("Player {} tried to walk to {:?} which is a hitbox", player_id, target_coordinates);
            return Err("Can't walk through here!".into());
        }
    }

    /* Terrain validation currently gets triggered by HTM + glancing, so it's disabled
    //Only do terrain-related validation for deployables now
    if movement_type != MovementType::Amphibious {
        let terrain_source = unwrap_or_err!(TerrainChunkState::get_terrain_cell(ctx, &source_large), "Invalid source location");
        let terrain_target = unwrap_or_err!(TerrainChunkState::get_terrain_cell(ctx, &target_large), "Invalid destination");
        let target_should_swim = terrain_target.player_should_swim();
        let target_is_submerged = target_should_swim || terrain_target.is_submerged();
        let source_is_submerged = terrain_source.is_submerged();

        if movement_type != MovementType::Amphibious {
            if target_should_swim && movement_type == MovementType::Ground {
                //Ground deployables can move in shallow water
                log::warn!(
                    "Player {} is driving a ground deployable and tried to walk to {:?} which is too deep",
                    player_id,
                    target_coordinates
                );
                return Err("Can't move on water.".into());
            }
            if !target_is_submerged && movement_type == MovementType::Water {
                log::warn!(
                    "Player {} is driving a water deployable and tried to walk to {:?} which is not submerged",
                    player_id,
                    target_coordinates
                );
                return Err("Can't move on land.".into());
            }
        }

        //Prevent movement over elevation
        if movement_type == MovementType::Water {
            let elevation_diff = i32::abs(terrain_source.water_level - terrain_target.water_level);
            if elevation_diff > max_elevation_water {
                log::warn!(
                    "Player {} is driving a water deployable and tried to walk from {:?} to {:?} which is an elevation diff of {}",
                    player_id,
                    source_coordinates,
                    target_coordinates,
                    elevation_diff
                );
                return Err("Can't swim here".into());
            }
        } else if source_is_submerged & target_should_swim {
            let elevation_diff = i32::abs(terrain_source.water_level - terrain_target.water_level);
            if elevation_diff > max_elevation_water {
                log::warn!(
                    "Player {} tried to swim from {:?} to {:?} which is an elevation diff of {}",
                    player_id,
                    source_coordinates,
                    target_coordinates,
                    elevation_diff
                );
                return Err("Can't swim here".into());
            }
        } else {
            let elevation_diff = i32::abs(terrain_source.player_surface_elevation() - terrain_target.player_surface_elevation());
            if elevation_diff > max_elevation_land {
                log::warn!(
                    "Player {} tried to walk from {:?} to {:?} which is an elevation diff of {}",
                    player_id,
                    source_coordinates,
                    target_coordinates,
                    elevation_diff
                );
                return Err("Can't walk here".into());
            }
        }
    }*/

    //let estimated_duration = travel_time(source_coordinates, target_coordinates, move_speed);
    //if duration < estimated_duration * 0.9 {
    //    log::warn!(
    //        "Player {} tried to move too quickly from {:?} to {:?} (estimated duration: {}, received duration: {})",
    //        player_id,
    //        source_coordinates,
    //        target_coordinates,
    //        estimated_duration,
    //        duration
    //    );
    //    return Err("Tried to move too quickly".into());
    //}

    Ok(())
}

pub fn travel_time(source_coordinates: &FloatHexTile, target_coordinates: &FloatHexTile, speed: f32) -> f32 {
    let distance = (source_coordinates.to_world_position() - target_coordinates.to_world_position()).magnitude();
    distance / speed
}

pub fn validate_move_origin(
    prev_origin: &FloatHexTile,
    cur_origin: &FloatHexTile,
    timestamp_diff_ms: u64,
    move_speed: f32,
    player_id: u64,
) -> Result<(), String> {
    const DURATION_LENIENCY_FLAT_VALUE: f32 = 0.05;
    const DURATION_LENIENCY_MULTIPLIER: f32 = 0.9;

    let estimated_duration = travel_time(prev_origin, cur_origin, move_speed);
    let timestamp_diff = timestamp_diff_ms as f32 / 1000.0;
    if timestamp_diff < estimated_duration * DURATION_LENIENCY_MULTIPLIER - DURATION_LENIENCY_FLAT_VALUE {
        log::warn!(
            "Player {} tried to move too quickly from {} to {} (estimated duration: {}, received duration: {})",
            player_id,
            prev_origin,
            cur_origin,
            estimated_duration,
            timestamp_diff
        );
        return Err("~Tried to move too quickly".into());
    }

    Ok(())
}

pub fn validate_move_basic(
    ctx: &ReducerContext,
    prev_origin: &FloatHexTile,
    source_coordinates: &FloatHexTile,
    target_coordinates: &FloatHexTile,
    duration: f32,
) -> Result<(), String> {
    //Blatant cheating checks
    const MAX_DURATION: f32 = 100.0;
    const MAX_DISTANCE: f32 = 7.0; //Realistically should never exceed 3
    const MAX_DISTANCE_FROM_PREV_STATE: f32 = 7.0;
    const MAX_SPEED: f32 = 100.0;
    if !(0.0..=MAX_DURATION).contains(&duration) {
        return Err("Invalid duration.".into());
    }
    if source_coordinates.dimension != target_coordinates.dimension {
        return Err("Invalid dimension.".into());
    }

    if target_coordinates.dimension != prev_origin.dimension {
        return Err("Client sent wrong dimension".into());
    }

    if target_coordinates.distance_to(*source_coordinates) > MAX_DISTANCE {
        return Err("Can't move that far".into());
    }

    if prev_origin.distance_to(*source_coordinates) > MAX_DISTANCE_FROM_PREV_STATE {
        return Err("Can't move that far".into());
    }

    if target_coordinates.dimension != dimensions::OVERWORLD
        && !game_state_filters::is_interior_tile_walkable(ctx, target_coordinates.parent_small_tile())
    {
        return Err("Can't move outside interior bounds".into());
    }

    let distance = (target_coordinates.to_world_position() - source_coordinates.to_world_position()).magnitude();
    if duration == 0.0 {
        if distance > 0.0 {
            return Err("Can't move that fast".into());
        }
    } else {
        let speed = distance / duration;
        if speed > MAX_SPEED {
            return Err("Can't move that fast".into());
        }
    }

    Ok(())
}

pub fn validate_move_timestamp(prev_timestamp: u64, received_timestamp: u64, now: Timestamp) -> Result<(), String> {
    //Allow some leniency since clients don't currently have accurate ServerTime
    const MAX_OFFSET_INTO_PAST_MS: i64 = 8000; //Allow processing requests "from the past" (client sent a request that got delayed)
    const MAX_OFFSET_INTO_FUTURE_MS: i64 = 1000; //Allow receiving requests "from the future" (client recovering after a lag spike)

    let prev_timestamp = prev_timestamp as i64;
    let received_timestamp = received_timestamp as i64;
    let now_ms = game_state::unix_ms(now) as i64;
    if received_timestamp - now_ms > MAX_OFFSET_INTO_FUTURE_MS {
        log::warn!(
            "Invalid timestamp: too far into the future. Current time: {}, received time: {}",
            now_ms,
            received_timestamp
        );
        return Err("~Invalid timestamp".into());
    }

    if now_ms - received_timestamp > MAX_OFFSET_INTO_PAST_MS {
        log::warn!(
            "Invalid timestamp: too far in the past. Current time: {}, received time: {}",
            now_ms,
            received_timestamp
        );
        return Err("~Invalid timestamp".into());
    }

    if received_timestamp < prev_timestamp {
        log::warn!(
            "Invalid timestamp: previous timestamp is more recent. Current time: {}, previous timestamp: {}, received time: {}",
            now_ms,
            prev_timestamp,
            received_timestamp
        );
        return Err("~Invalid timestamp".into());
    }

    Ok(())
}

//Leaving this here for when we re-enable more comprehensivee move validation
//#[allow(dead_code)]
//pub fn validate_move_old(
//    source_coordinates: &FloatHexTile,
//    target_coordinates: &FloatHexTile,
//    max_elevation: i32,
//    movement_type: MovementType,
//) -> Result<(), String> {
//    let mut error: String = "".into();
//
//    let can_move_on_tile = |tile: SmallHexTile, error: &mut String| -> bool {
//        // Can't move onto "hitbox" footprints
//        if game_state_filters::has_hitbox_footprint(tile) {
//            *error = "Can't walk through here!".into();
//            return false;
//        }
//
//        if movement_type != MovementType::Amphibious {
//            let terrain = TerrainChunkState::get_terrain_cell(ctx, &tile.parent_large_tile()).unwrap();
//            let is_submerged = terrain.is_submerged();
//            if is_submerged && movement_type == MovementType::Ground {
//                *error = "Can't move on water.".into();
//                return false;
//            }
//            if !is_submerged && movement_type == MovementType::Water {
//                *error = "Can't move on ground.".into();
//                return false;
//            }
//        }
//
//        return true;
//    };
//
//    let mut can_move_on_next_tile = !can_move_on_tile(source_coordinates.clone().into(), &mut error); //Check to make sure players don't get stuck inside buildings
//
//    let can_transition = |tile_from: SmallHexTile, tile_to: SmallHexTile| -> bool {
//        if !can_move_on_tile(tile_to, &mut error) {
//            return can_move_on_next_tile;
//        }
//
//        //Prevent movement over elevation (unless in water - todo if we have waterfalls)
//        if movement_type == MovementType::Ground {
//            let terrain_source = unwrap_or_err!(TerrainChunkState::get_terrain_cell(ctx, &tile_from.parent_large_tile()), "Invalid source location");
//            let terrain_target = unwrap_or_err!(TerrainChunkState::get_terrain_cell(ctx, &tile_to.parent_large_tile()), "Invalid destination");
//            let elevation_diff = i32::abs(terrain_source.elevation - terrain_target.elevation);
//            if elevation_diff > max_elevation {
//                error = "Pathfinding error - move handler trying to climb.".into();
//                return can_move_on_next_tile;
//            }
//        }
//
//        can_move_on_next_tile = false;
//        return true;
//    };
//
//    if !raycast(source_coordinates, target_coordinates, can_transition) {
//        return Err(error.to_string());
//    }
//
//    Ok(())
//}

pub fn raycast<F: FnMut(SmallHexTile, SmallHexTile) -> bool>(
    start_tile: &FloatHexTile,
    end_tile: &FloatHexTile,
    mut can_transition: F,
) -> bool {
    //Port of client's HexTerrain.RaycastOnGrid
    //Assumes tiles are circles (outer hex radius) and runs ray-circle intersection on rays that always originate inside circles

    let start = start_tile.to_world_position();
    let end = end_tile.to_world_position();

    let mut start_tile = SmallHexTile::from_position(start, start_tile.dimension);
    let mut orig = start;
    let diff = end - start;
    let magnitude = diff.magnitude();
    let dir = diff / magnitude;
    let radius2 = coordinates::consts::OUTER_RADIUS * coordinates::consts::OUTER_RADIUS;
    let mut processed_magnitude = 0.0;

    let mut _i = 0;
    loop {
        let center = start_tile.to_center_position_xz();

        //https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-sphere-intersection
        // geometric solution
        let l = center - orig;
        let tca = Vector2::dot(&l, &dir);
        // if (tca < 0) return false;
        let d = l.sqr_magnitude() - tca * tca;
        // if (d > radius2) return false; //Our ray starts inside the circle so there's always 1 intersection point
        let thc = (radius2 - d).sqrt();

        //Solutions
        //Since ray starts inside the circle one point is guaranteed to be behind the ray and one in front of it
        //float t0 = tca - thc; //This is always behind ray in our case
        let t1 = tca + thc;
        let max = t1 + 0.01; //Ensure we're getting next tile

        processed_magnitude += max;
        if processed_magnitude > magnitude {
            break;
        }

        orig = start + dir * processed_magnitude;
        let next_tile = SmallHexTile::from_position(orig, start_tile.dimension);
        if !can_transition(start_tile, next_tile) {
            return false;
        }
        start_tile = next_tile;
        _i += 1;
    }

    true
}

pub fn move_validation_strike(
    ctx: &ReducerContext,
    actor_id: u64,
    entity_id_to_reset: u64,
    prev_origin: FloatHexTile,
    identifier: String,
    error: String,
) -> Result<(), String> {
    if let Err(strike_counter) = validation_strike(ctx, actor_id, identifier, "move".into()) {
        return fail_validation(ctx, error, entity_id_to_reset, prev_origin, Some(strike_counter));
    }

    Ok(())
}

pub fn action_validation_strike(ctx: &ReducerContext, actor_id: u64, action_type: PlayerActionType) -> Result<(), String> {
    if validation_strike(ctx, actor_id, format!("Player {actor_id}"), format!("{:?}", action_type)).is_err() {
        return player_action_helpers::fail_timing(ctx, actor_id, action_type, format!("Tried to {{0}} too quickly|~{:?}", action_type));
    }

    Ok(())
}

pub fn fail_validation(
    ctx: &ReducerContext,
    error: String,
    entity_id: u64,
    coord: FloatHexTile,
    strike_counter: Option<MoveValidationStrikeCounterState>,
) -> Result<(), String> {
    let oc: OffsetCoordinatesFloat = coord.into();
    ctx.db
        .reset_mobile_entity_timer()
        .try_insert(ResetMobileEntityTimer {
            scheduled_id: 0,
            scheduled_at: ctx.timestamp.into(),
            owner_entity_id: entity_id,
            position: Some(oc),
            strike_counter_to_update: strike_counter,
        })
        .ok()
        .unwrap();
    Err(error)
}

pub fn validation_strike(
    ctx: &ReducerContext,
    actor_id: u64,
    identifier: String,
    action: String,
) -> Result<(), MoveValidationStrikeCounterState> {
    let params = ctx.db.private_parameters_desc().version().find(0).unwrap();
    let mut strike_counter = ctx.db.move_validation_strike_counter_state().entity_id().find(actor_id).unwrap();

    let oldest_timestamp = Timestamp::from_micros_since_unix_epoch(
        ctx.timestamp.to_time_duration_since_unix_epoch().to_micros()
            - (params.move_validation.strike_counter_time_window_sec * 1_000_000) as i64,
    );
    strike_counter.validation_failure_timestamps.retain(|t| *t > oldest_timestamp); //Remove old timestamps
    strike_counter.validation_failure_timestamps.push(ctx.timestamp);

    let cur_strikes = strike_counter.validation_failure_timestamps.len() as i32;
    let max_strikes = params.move_validation.strike_count_before_move_validation_failure;
    if cur_strikes > max_strikes {
        log::error!(
            "{} failed {} validation, the request is rejected (strike {}/{})",
            identifier,
            action,
            cur_strikes,
            max_strikes,
        );
        return Err(strike_counter);
    } else {
        log::warn!(
            "{} failed {} validation, but is allowed to proceed (strike {}/{})",
            identifier,
            action,
            cur_strikes,
            max_strikes,
        );
        ctx.db.move_validation_strike_counter_state().entity_id().update(strike_counter);
    }

    Ok(())
}
