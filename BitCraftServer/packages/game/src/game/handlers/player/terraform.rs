use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::messages::components::PlayerActionState;
use crate::messages::util::OffsetCoordinatesLargeMessage;
use crate::{
    game::{coordinates::ChunkCoordinates, game_state::game_state_filters},
    messages::{action_request::PlayerTerraformRequest, components::*, static_data::*},
    unwrap_or_err,
};
use bitcraft_macro::feature_gate;
use spacetimedb::{log, ReducerContext};
use std::time::Duration;

fn event_delay_recipe_id(ctx: &ReducerContext, terrain_cell: Option<TerrainCell>, actor_id: u64, delta: i16) -> (Duration, Option<i32>) {
    if terrain_cell.is_none() {
        return (Duration::ZERO, None);
    }

    let terrain_cell = terrain_cell.unwrap();

    let target = terrain_cell.elevation + delta;

    let height_difference = i16::min(
        i16::abs(target - terrain_cell.original_elevation),
        TerraformRecipeDesc::max_difference(ctx),
    );
    let recipe = ctx.db.terraform_recipe_desc().difference().find(&height_difference);
    if recipe.is_none() {
        return (Duration::ZERO, None);
    }

    let recipe = recipe.unwrap();
    let mut time = recipe.time_per_action;
    if let Some(stats) = ctx.db.character_stats_state().entity_id().find(&actor_id) {
        let skill_speed = stats.get_skill_speed(SkillType::Construction);
        let time_multiplier = 1.0 / (stats.get(CharacterStatType::BuildingSpeed) + skill_speed - 1.0);
        time = time * time_multiplier;
    }
    return (Duration::from_secs_f32(time), Some(recipe.difference.into()));
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn terraform_start(ctx: &ReducerContext, request: PlayerTerraformRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut cache = TerrainChunkCache::empty();

    let target = get_terraform_building_id(ctx, request.coordinates);

    let building_entity_id = unwrap_or_err!(
        game_state_filters::building_id_at_coordinates(ctx, &game_state_filters::coordinates_any(ctx, actor_id)),
        "Player has to be in a building"
    );

    let terraform_progress_state = unwrap_or_err!(
        ctx.db.terraform_progress_state().entity_id().find(&building_entity_id),
        "terraform progress action not found"
    );

    let terrain_cell = cache.get_terrain_cell(ctx, &request.coordinates.into());
    let mut delta = 0;
    if let Some(t) = terrain_cell {
        if terraform_progress_state.final_height_target > t.elevation {
            delta = 1
        } else {
            delta = -1
        }
    }
    let (delay, recipe_id) = event_delay_recipe_id(ctx, terrain_cell, actor_id, delta);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Terraform,
        target,
        recipe_id,
        delay,
        reduce(ctx, &mut cache, actor_id, &request, true),
        request.timestamp,
    )
}

fn get_terraform_building_id(ctx: &ReducerContext, coordinates: OffsetCoordinatesLargeMessage) -> Option<u64> {
    let terrain_coordinates = LargeHexTile::from(coordinates);
    game_state_filters::building_id_at_coordinates(ctx, &terrain_coordinates.center_small_tile().into())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn terraform(ctx: &ReducerContext, request: PlayerTerraformRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Terraform.get_layer(ctx),
        reduce(ctx, &mut cache, actor_id, &request, false),
    )
}

fn reduce(
    ctx: &ReducerContext,
    cache: &mut TerrainChunkCache,
    actor_id: u64,
    request: &PlayerTerraformRequest,
    dry_run: bool,
) -> Result<(), String> {
    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::Terraform, request.timestamp)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(
            ctx,
            actor_id,
            PlayerActionType::Terraform,
            get_terraform_building_id(ctx, request.coordinates),
        )?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Terraform, request.timestamp)?;
    }

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let terrain_coordinates = request.coordinates;
    let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinates::from_terrain_coordinates(terrain_coordinates.into()));
    // The terrain coordinates in the request are already scaled (LargeHexTile).
    let terrain_chunk = unwrap_or_err!(cache.filter_by_chunk_index(ctx, chunk_index), "Invalid terrain chunk");

    let offset_coordinates = terrain_coordinates;
    let mut terrain_cell = unwrap_or_err!(terrain_chunk.get_entity(&offset_coordinates), "Invalid terrain chunk");

    let building_entity_id = unwrap_or_err!(
        game_state_filters::building_id_at_coordinates(ctx, &game_state_filters::coordinates_any(ctx, actor_id)),
        "Player has to be in a building"
    );

    let mut terraform_progress_state = unwrap_or_err!(
        ctx.db.terraform_progress_state().entity_id().find(&building_entity_id),
        "terraform progress action not found"
    );

    //We should return an error if a terraform action is started but the next_height_target is equal to the terrain elevation.
    if terraform_progress_state.next_height_target == terrain_cell.elevation {
        return Err("dig height is already equal to elevation".into());
    }

    //cancel resets progress to min value
    if !request.start_new && terraform_progress_state.progress < 0 {
        return Err("Terraform action cancelled".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id),
        "Invalid terraforming building"
    );

    //only claim members can dig
    terraform_progress_state.validate_permission_terraform(ctx, actor_id, building.claim_entity_id)?;

    let dir = if terraform_progress_state.next_height_target < terraform_progress_state.final_height_target {
        1
    } else {
        -1
    };

    if dir < 0 {
        if terrain_cell.water_level >= terrain_cell.elevation {
            return Err("Can't dig below water level".into());
        }

        if terrain_cell.elevation <= 1 {
            return Err("Can't dig that low".into());
        }

        for direction in HexDirection::FLAT {
            let neighbor_coordinates = LargeHexTile::from(terrain_coordinates).neighbor(direction);
            let neighbor = unwrap_or_err!(
                cache.get_terrain_cell(ctx, &neighbor_coordinates),
                "Can't dig so close to the world edge"
            );
            if neighbor.is_submerged() && neighbor.water_level >= terrain_cell.elevation {
                return Err("Can't dig next to water".into());
            }
        }
    }

    let player_coordinates = game_state_filters::coordinates_any(ctx, actor_id);

    if player_coordinates.parent_large_tile() != terrain_coordinates.into() {
        return Err("Invalid coordinates".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id),
        "Invalid terraforming building"
    );
    let building_description = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building.building_description_id),
        "Invalid building description"
    );

    if !building_description.has_category(ctx, BuildingCategory::TerrraformingBase) {
        return Err("Building isn't a terraforming base".into());
    }

    let target = terraform_progress_state.next_height_target;

    let height_difference = i16::min(
        i16::abs(target - terrain_cell.original_elevation),
        TerraformRecipeDesc::max_difference(ctx),
    );
    let recipe = unwrap_or_err!(
        ctx.db.terraform_recipe_desc().difference().find(&height_difference),
        "Recipe not found"
    );

    if terraform_progress_state.next_height_target >= terraform_progress_state.final_height_target
        && terrain_cell.elevation == terraform_progress_state.final_height_target
        && !request.start_new
    {
        return Err("Terraform is already completed".into());
    }

    let stamina = unwrap_or_err!(ctx.db.stamina_state().entity_id().find(&actor_id), "Player not found.");
    if stamina.stamina < recipe.stamina_per_action as f32 {
        return Err("Not enough stamina!".into());
    }

    let stats = ctx.db.character_stats_state().entity_id().find(actor_id).unwrap();
    let skill_power = stats.get_skill_power(SkillType::Construction);

    let tool_power = if recipe.tool_requirement.is_none() {
        1.0
    } else {
        let tool = match ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirement.unwrap()) {
            Ok(tool) => tool,
            Err(err_str) => return Err(err_str.into()),
        };
        tool.power as f32
    } + skill_power;

    let actions_count = tool_power.round() as i32;

    if !dry_run {
        StaminaState::add_player_stamina(ctx, actor_id, -recipe.stamina_per_action as f32);

        //cancel sets progress to min value. Make sure at least 0 when progressing
        if terraform_progress_state.progress < 0 {
            terraform_progress_state.progress = 0;
        }

        let mut spent_actions = actions_count.min(recipe.actions_count - terraform_progress_state.progress);
        if spent_actions < 0 {
            log::error!(
                "Terraform progress {} is greater than recipe actions count {}",
                terraform_progress_state.progress,
                recipe.actions_count
            );
            spent_actions = 0;
        }

        let mut output = Vec::new();
        if let Some(output_item_stacks) = &recipe.output_item_stacks {
            for stack in output_item_stacks {
                if let Some(rolled) = stack.roll(ctx, spent_actions) {
                    output.push(rolled);
                }
            }
        }

        terraform_progress_state.progress += actions_count;
        if terraform_progress_state.progress >= recipe.actions_count {
            terraform_progress_state.progress = 0;
            terraform_progress_state.next_height_target = terraform_progress_state.next_height_target + dir;
            terrain_cell.elevation = target;

            let terrain_chunk = cache.filter_by_chunk_index(ctx, chunk_index).expect("the chunk should still exist");
            terrain_chunk.set_entity(offset_coordinates, terrain_cell);
            cache.persist(ctx, chunk_index);

            //only finish when reach final target
            if terrain_cell.elevation == terraform_progress_state.final_height_target {
                PlayerActionState::success(
                    ctx,
                    actor_id,
                    PlayerActionType::None,
                    PlayerActionType::Terraform.get_layer(ctx),
                    0,
                    None,
                    None,
                    request.timestamp,
                );
            }
        }
        ctx.db.terraform_progress_state().entity_id().update(terraform_progress_state);

        let experience_per_progress = ctx
            .db
            .parameters_desc()
            .version()
            .find(0)
            .unwrap()
            .terraform_experience_per_progress;
        let construction_skill_id = SkillType::Construction as i32;
        let experience_gain = f32::ceil(experience_per_progress * spent_actions as f32) as i32;
        ExperienceState::add_experience(ctx, actor_id, construction_skill_id, experience_gain);

        if spent_actions > 0 {
            EquipmentState::try_activate_profession_hit_buffs(ctx, actor_id, Some(SkillType::Construction))?;
        }

        if !output.is_empty() {
            let terrain_drop_location = LargeHexTile::from(terrain_coordinates).center_small_tile();
            InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                ctx,
                actor_id,
                &output,
                |x| building.distance_to(ctx, &x),
                true,
                || vec![terrain_drop_location],
                false,
            )?;
        }

        PlayerActionState::mark_as_consumed(ctx, actor_id)?;
    }

    Ok(())
}
