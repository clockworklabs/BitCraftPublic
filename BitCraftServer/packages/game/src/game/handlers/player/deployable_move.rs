use crate::game::game_state::{game_state_filters, wind_system};
use crate::game::handlers::authentication::has_role_no_dev;
use crate::game::reducer_helpers;
use crate::messages::authentication::Role;
use crate::messages::util::FloatHexTileMessage;
use crate::{
    game::{
        coordinates::*, entities::location::MobileEntityState, game_state, reducer_helpers::move_validation_helpers,
        terrain_chunk::TerrainChunkCache,
    },
    messages::{action_request::PlayerDeployableMoveRequest, components::*, static_data::*},
    unwrap_or_err,
};
use crate::messages::events::*;
use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

const FULL_WIND_BONUS_ANGLE_RAD: f32 = std::f32::consts::PI / 3.0;
const PARTIAL_WIND_BONUS_ANGLE_RAD: f32 = std::f32::consts::PI * 2.0 / 3.0;

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_move(ctx: &ReducerContext, request: PlayerDeployableMoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mounting = unwrap_or_err!(
        ctx.db.mounting_state().entity_id().find(actor_id),
        "Player is not in a deployable!"
    ); //*DEPLOYABLE*//

    if mounting.deployable_slot != 0 {
        return Err("Player is not the one controlling the deployable".into());
    }

    let deployable_entity_id = mounting.deployable_entity_id;

    let prev_mobile_entity = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(deployable_entity_id),
        "Deployable is no longer available"
    );

    let prev_origin = prev_mobile_entity.coordinates_float();
    let origin = unwrap_or_err!(request.origin, "Expected origin in move request");
    let dest = unwrap_or_err!(request.destination, "Expected destination in move request");
    let source_coordinates: FloatHexTile = origin.into();
    let target_coordinates: FloatHexTile = dest.into();

    let paving = PavedTileState::get_at_location(ctx, &prev_origin.parent_small_tile())
        .map(|t| ctx.db.paving_tile_desc().id().find(t.tile_type_id).unwrap());

    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_prev = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &prev_origin.parent_large_tile()),
        "You can't go here!",
    );
    let terrain_start = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &source_coordinates.parent_large_tile()),
        "You can't go here!",
    );
    let terrain_target = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &target_coordinates.parent_large_tile()),
        "You can't go here!",
    );

    // Validate Boat Move. Note that this for the short term, ideally at some point
    // we will re-enable the above _validate_move that has more comprehensive checks
    let deployable = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id),
        "Deployable not found!"
    );
    let desc = unwrap_or_err!(
        ctx.db.deployable_desc().id().find(deployable.deployable_description_id),
        "Invalid deployable type"
    );

    let water_body_type = if terrain_start.is_submerged() {
        terrain_start.water_body_type
    } else {
        SurfaceType::Ground as u8
    };
    let speed = game_state_filters::get_speed_on_water_type(&desc.speed, water_body_type, None, false);

    if !has_role_no_dev(ctx, &ctx.sender(), Role::Gm) {
        move_validation_helpers::validate_move_timestamp(prev_mobile_entity.timestamp, request.timestamp, ctx.timestamp)?;
        move_validation_helpers::validate_move_basic(
            ctx,
            &prev_mobile_entity.coordinates_float(),
            &source_coordinates,
            &target_coordinates,
            request.duration,
        )?;
        validate_move(
            ctx,
            actor_id,
            speed,
            deployable_entity_id,
            &prev_mobile_entity,
            &request,
            source_coordinates,
            target_coordinates,
            &paving,
            &desc,
        )?;
    }

    let pathfinding = unwrap_or_err!(
        ctx.db.pathfinding_desc().id().find(desc.pathfinding_id),
        "Invalid deployable pathfinding info"
    );

    let prev_water_depth = terrain_prev.water_depth() as i32;
    let start_water_depth = terrain_start.water_depth() as i32;
    let target_water_depth = terrain_target.water_depth() as i32;

    if (start_water_depth < pathfinding.min_water_depth || start_water_depth > pathfinding.max_water_depth)
        && (target_water_depth < pathfinding.min_water_depth || target_water_depth > pathfinding.max_water_depth)
    {
        return Err("You can't go here!".into());
    }

    //Make sure players aren't phasing through cliffs
    if prev_water_depth > 0 && start_water_depth > 0 {
        let delta = if desc.deployable_type == DeployableType::Boat {
            let prev_surface_level = terrain_prev.surface_level() as i32;
            let start_surface_level = terrain_start.surface_level() as i32;
            (prev_surface_level - start_surface_level).abs()
        } else {
            let prev_surface_level = terrain_prev.player_surface_elevation() as i32;
            let start_surface_level = terrain_start.player_surface_elevation() as i32;
            (prev_surface_level - start_surface_level).abs()
        };
        if delta > pathfinding.max_swim_height_delta {
            return Err("~Origin water level mismatch".into());
        }
    } else {
        let elevation_diff = (terrain_prev.player_surface_elevation() - terrain_start.player_surface_elevation()) as i32;
        if (elevation_diff > 0
            && elevation_diff
                > pathfinding
                    .climb_up_options
                    .iter()
                    .map(|c| c.max_elevation_difference)
                    .max()
                    .unwrap_or(5))
            || (elevation_diff < 0
                && elevation_diff
                    < pathfinding
                        .climb_down_options
                        .iter()
                        .map(|c| c.max_elevation_difference)
                        .min()
                        .unwrap_or(-5))
        {
            return Err("~Origin elevation mismatch".into());
        }
    }

    let distance_moved = prev_origin.distance_to(source_coordinates);
    let experience_per_progress = desc.experience_per_progress.first();
    if let Some(experience_per_progress) = experience_per_progress {
        let quantity = experience_per_progress.quantity * distance_moved;
        ExperienceState::add_experience_f32(ctx, actor_id, experience_per_progress.skill_id, quantity);
    }

    // update deployable location
    reducer_helpers::deployable_helpers::move_deployable(ctx, deployable_entity_id, origin, dest, request.timestamp, request.duration)?;
    ctx.db.deployable_move_event().insert(DeployableMoveEvent {
        actor_id,
        request,
        is_follow: false,
    });
    Ok(())
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn validate_move(
    ctx: &ReducerContext,
    actor_id: u64,
    speed: f32,
    deployable_entity_id: u64,
    prev_mobile_entity: &MobileEntityState,
    request: &PlayerDeployableMoveRequest,
    source_coordinates: FloatHexTile,
    _target_coordinates: FloatHexTile,
    paving: &Option<PavingTileDesc>,
    deployable_desc: &DeployableDesc,
) -> Result<(), String> {
    let prev_origin = prev_mobile_entity.coordinates_float();

    if source_coordinates.x != prev_origin.x || source_coordinates.z != prev_origin.z {
        let move_speed = get_move_speed(
            ctx,
            actor_id,
            speed,
            prev_mobile_entity,
            source_coordinates,
            paving,
            deployable_desc,
            prev_origin,
        );

        //let (cur_position, cur_distance) = prev_mobile_entity.cur_coord_and_distance_traveled(move_speed);

        let timestamp_diff_ms = request.timestamp - prev_mobile_entity.timestamp;
        if let Err(error) =
            move_validation_helpers::validate_move_origin(&prev_origin, &source_coordinates, timestamp_diff_ms, move_speed, actor_id)
        {
            //Can return Err or Ok
            return move_validation_helpers::move_validation_strike(
                ctx,
                actor_id,
                deployable_entity_id,
                prev_origin,
                format!("Player {{0}} (vehicle {{1}})|~{actor_id}|~{deployable_entity_id}"),
                error,
            );
        }

        //let par = ctx.db.parameters_desc().version().find(&0).unwrap();
        //let result = move_validation_helpers::validate_move(
        //    &prev_mobile_entity,
        //    &prev_origin,
        //    &source_coordinates,
        //    &target_coordinates,
        //    par.player_jump_height as i32,
        //    par.player_swim_height as i32,
        //    deployable_desc.movement_type,
        //    speed,
        //    speed,
        //    duration,
        //    actor_id,
        //);
    }

    Ok(())
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn get_move_speed(
    ctx: &ReducerContext,
    actor_id: u64,
    base_speed: f32,
    prev_mobile_entity: &MobileEntityState,
    source_coordinates: FloatHexTileMessage,
    paving: &Option<PavingTileDesc>,
    deployable_desc: &DeployableDesc,
    prev_origin: FloatHexTileMessage,
) -> f32 {
    let deployable_bonus = CharacterStatsState::get_deployable_speed_multiplier(ctx, actor_id, deployable_desc.deployable_type) - 1f32;
    let mut player_bonus = 0f32;
    let mut pavement_bonus = 0f32;
    let mut wind_bonus = 0f32;

    if deployable_desc.use_player_speed_modifier {
        player_bonus = PlayerState::get_stat(ctx, actor_id, CharacterStatType::MovementMultiplier) - 1f32;
    }

    if let Some(paving) = paving {
        pavement_bonus = paving.apply_stat_to_value_unclamped(1f32, crate::CharacterStatType::MovementMultiplier) - 1f32;
    }

    if deployable_desc.affected_by_wind.abs() > 0.01 {
        let wind_angle = wind_system::sample_wind_float(ctx, &source_coordinates, Some(prev_mobile_entity.timestamp));
        let direction = source_coordinates.to_world_position() - prev_origin.to_world_position();
        let player_angle = f32::atan2(direction.y, direction.x);
        let mut angle = (player_angle - wind_angle) % (std::f32::consts::PI * 2.0);
        if angle > std::f32::consts::PI {
            angle -= std::f32::consts::PI * 2.0;
        } else if angle < -std::f32::consts::PI {
            angle += std::f32::consts::PI * 2.0;
        }

        let angle = angle.abs();
        if angle < FULL_WIND_BONUS_ANGLE_RAD {
            //(.5 + SailingLevel / 2)%
            let sailing_lvl = ctx
                .db
                .experience_state()
                .entity_id()
                .find(actor_id)
                .expect("Player has no EpxerienceState")
                .get_level(SkillType::Sailing as i32);
            wind_bonus = (0.4 + sailing_lvl as f32 / 100.0 * 0.4) * deployable_desc.affected_by_wind;
        } else if angle < PARTIAL_WIND_BONUS_ANGLE_RAD {
            //(.5 + SailingLevel / 4)%
            let sailing_lvl = ctx
                .db
                .experience_state()
                .entity_id()
                .find(actor_id)
                .expect("Player has no EpxerienceState")
                .get_level(SkillType::Sailing as i32);
            wind_bonus = (0.2 + sailing_lvl as f32 / 100.0 * 0.2) * deployable_desc.affected_by_wind;
        }
    }

    base_speed * (1f32 + deployable_bonus + player_bonus + pavement_bonus + wind_bonus)
}
