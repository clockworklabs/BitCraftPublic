use bitcraft_macro::feature_gate;
use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::game::reducer_helpers::building_helpers::{create_building_spawns, create_distant_visibile_building};
use crate::{
    game::{
        discovery::Discovery,
        entities::{building_state::BuildingState, location::LocationState},
        game_state::{self, game_state_filters},
        permission_helper,
        reducer_helpers::{
            building_helpers::{create_building_claim, create_building_component},
            footprint_helpers,
            interior_helpers::create_building_interior,
            player_action_helpers,
        },
        terrain_chunk::TerrainChunkCache,
    },
    messages::{action_request::PlayerProjectSiteAdvanceProjectRequest, components::*, static_data::*},
    unwrap_or_err,
};

pub fn event_delay_recipe_id(
    ctx: &ReducerContext,
    request: &PlayerProjectSiteAdvanceProjectRequest,
    stats: &CharacterStatsState,
) -> (Duration, Option<i32>) {
    let project_site = match ctx.db.project_site_state().entity_id().find(&request.owner_entity_id) {
        Some(cs) => cs,
        None => return (Duration::ZERO, None),
    };

    let mut time_requirement = 0.0;
    let mut skill_speed = 1.0;
    let recipe_id: Option<i32>;
    let construction_recipe = ctx.db.construction_recipe_desc().id().find(&project_site.construction_recipe_id);
    if let Some(recipe) = construction_recipe {
        time_requirement = recipe.time_requirement;
        recipe_id = Some(project_site.construction_recipe_id);
        if let Some(skill) = recipe.get_skill_type() {
            skill_speed = stats.get_skill_speed(skill);
        }
    } else {
        let resource_placement_recipe = ctx
            .db
            .resource_placement_recipe_desc()
            .id()
            .find(&project_site.resource_placement_recipe_id);
        recipe_id = Some(project_site.resource_placement_recipe_id);
        if let Some(recipe) = resource_placement_recipe {
            time_requirement = recipe.time_requirement;
        }
    }

    let build_time_multiplier = 1.0 / (stats.get(CharacterStatType::BuildingSpeed) + skill_speed - 1.0);
    return (Duration::from_secs_f32(time_requirement * build_time_multiplier), recipe_id);
}

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn project_site_advance_project_start(ctx: &ReducerContext, request: PlayerProjectSiteAdvanceProjectRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    let target = Some(request.owner_entity_id);
    let (delay, recipe_id) = event_delay_recipe_id(ctx, &request, &stats);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Build,
        target,
        recipe_id,
        delay,
        reduce(ctx, actor_id, request, stats, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[shared_table_reducer] //For waystones and claims
#[feature_gate("build")]
pub fn project_site_advance_project(ctx: &ReducerContext, request: PlayerProjectSiteAdvanceProjectRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Build.get_layer(ctx),
        reduce(ctx, actor_id, request, stats, false),
    )
}

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    request: PlayerProjectSiteAdvanceProjectRequest,
    stats: CharacterStatsState,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::Build, request.timestamp)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Build, Some(request.owner_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Build, request.timestamp)?;
    }

    let mut project_site = unwrap_or_err!(
        ctx.db.project_site_state().entity_id().find(&request.owner_entity_id),
        "Invalid project site"
    )
    .clone();

    let location: LocationState = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(&request.owner_entity_id),
        "Invalid project site"
    )
    .clone();

    let coordinates = location.coordinates();

    if !PermissionState::can_interact_with_tile(ctx, actor_id, coordinates, Permission::Usage) {
        return Err("You don't have permission to interact with this project site".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, coordinates, actor_id, ClaimPermission::Usage) {
        return Err("You don't have permission to interact with this project site".into());
    }

    // Somewhat gross, but construction and resource placement recipes share those variables that are used by the project site.
    let mut stamina_requirement = 0.0;
    let mut tool_requirements = Vec::new();
    let mut level_requirements = Vec::new();
    let mut actions_required = 0;
    let mut consumed_item_stacks = Vec::new();
    let mut consumed_cargo_stacks = Vec::new();
    let mut experience_per_progress = Vec::new();
    let mut building = None;
    let mut resource = None;
    let mut skill = None;

    if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(&project_site.construction_recipe_id) {
        skill = recipe.get_skill_type();
        stamina_requirement = recipe.stamina_requirement as f32;
        tool_requirements = recipe.tool_requirements;
        level_requirements = recipe.level_requirements;
        actions_required = recipe.actions_required;
        consumed_item_stacks = recipe.consumed_item_stacks;
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        building = ctx.db.building_desc().id().find(&recipe.building_description_id);
        experience_per_progress = recipe.experience_per_progress;

        if recipe.required_claim_tech_ids.len() != 0 {
            let claim_tech = unwrap_or_err!(
                ctx.db.claim_tech_state().entity_id().find(&project_site.owner_id),
                "This claim is missing its tech tree"
            );
            for required_claim_tech_id in &recipe.required_claim_tech_ids {
                // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
                if !claim_tech.has_unlocked_tech(*required_claim_tech_id) {
                    return Err("Missing claim upgrades".into());
                }
            }
        }
    }
    if let Some(recipe) = ctx
        .db
        .resource_placement_recipe_desc()
        .id()
        .find(&project_site.resource_placement_recipe_id)
    {
        skill = recipe.get_skill_type();
        stamina_requirement = recipe.stamina_requirement as f32;
        tool_requirements = recipe.tool_requirements;
        level_requirements = recipe.level_requirements;
        actions_required = recipe.actions_required;
        consumed_item_stacks = recipe.consumed_item_stacks;
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        resource = ctx.db.resource_desc().id().find(&recipe.resource_description_id);
        experience_per_progress = recipe.experience_per_progress;

        if recipe.required_claim_tech_ids.len() != 0 {
            let claim_tech = unwrap_or_err!(
                ctx.db.claim_tech_state().entity_id().find(&project_site.owner_id),
                "This claim is missing its tech tree"
            );
            for required_claim_tech_id in &recipe.required_claim_tech_ids {
                // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
                if !claim_tech.has_unlocked_tech(*required_claim_tech_id) {
                    return Err("Missing claim upgrades".into());
                }
            }
        }
    }

    let stamina_state = unwrap_or_err!(
        ctx.db.stamina_state().entity_id().find(&actor_id),
        "Player missing stamina component!"
    );
    if stamina_state.stamina < stamina_requirement {
        return Err("Not enough stamina.".into());
    }

    if !dry_run {
        StaminaState::add_player_stamina(ctx, actor_id, -stamina_requirement);
    }

    // Validate Tool Requirement
    if tool_requirements.len() > 0 {
        if let Err(err_str) = ToolDesc::get_required_tool(ctx, actor_id, &tool_requirements[0]) {
            return Err(err_str.into());
        }
    }

    let actor_coords: crate::messages::util::FloatHexTileMessage = game_state_filters::coordinates_float(ctx, actor_id);
    if project_site.distance_to(ctx, actor_coords.parent_small_tile()) > 2 {
        return Err("Too far".into());
    }

    if let Some(ref building_desc) = building {
        let claim_entity_id = project_site.owner_id;
        if building_desc.has_category(ctx, BuildingCategory::Bank) {
            if ctx.db.bank_state().claim_entity_id().filter(claim_entity_id).next().is_some() {
                return Err("Claims can only have one bank".into());
            }
        }
        if building_desc.has_category(ctx, BuildingCategory::TownMarket) {
            if ctx
                .db
                .marketplace_state()
                .claim_entity_id()
                .filter(claim_entity_id)
                .next()
                .is_some()
            {
                return Err("Claims can only have one market".into());
            }
        }
    }

    let mut terrain_cache = TerrainChunkCache::empty();

    //Assumes the whole building is on the same elevation
    let target_coords = game_state_filters::coordinates(ctx, request.owner_entity_id);
    player_action_helpers::validate_action_elevation(ctx, &mut terrain_cache, actor_coords, target_coords, false, 4, "build")?;

    if project_site.progress >= actions_required {
        return Err("Invalid build action".into());
    }

    let items_target: i32 = consumed_item_stacks.iter().map(|s| s.quantity).sum();
    if items_target < 0 {
        return Err("Invalid materials requirements".into());
    }
    let cargos_target: i32 = consumed_cargo_stacks.iter().map(|s| s.quantity).sum();
    if cargos_target < 0 {
        return Err("Invalid materials requirements".into());
    }

    let items_count: i32 = project_site.items.iter().map(|s| s.quantity).sum();
    let cargos_count: i32 = project_site.cargos.iter().map(|s| s.quantity).sum();

    let target = items_target + cargos_target;

    let count = items_count + cargos_count;
    let max_progress = i32::clamp(
        f32::floor(((count as f32) / (target as f32)) * (actions_required as f32)) as i32,
        0,
        actions_required,
    );

    if project_site.progress >= max_progress {
        return Err("Add more materials to build".into());
    }

    let player_level = ctx
        .db
        .experience_state()
        .entity_id()
        .find(&actor_id)
        .unwrap()
        .get_level(level_requirements[0].skill_id);
    let recipe_desired_skill = level_requirements[0].level;

    let mut crit_multiplier = 1.0;
    let mut experience_actions_count = 0;
    let mut actions_count = 0;

    if player_level >= recipe_desired_skill {
        let skill_power = match skill {
            Some(skill) => stats.get_skill_power(skill),
            None => 0.0,
        };
        let tool_power = if tool_requirements.is_empty() {
            1.0
        } else {
            let tool = match ToolDesc::get_required_tool(ctx, actor_id, &tool_requirements[0]) {
                Ok(tool) => tool,
                Err(err_str) => return Err(err_str.into()),
            };
            tool.power as f32
        } + skill_power;

        crit_multiplier = stats.get_final_crit_multiplier(ctx, skill);
        let base_damage = tool_power.round() as i32;
        let damage = (tool_power * crit_multiplier).round() as i32;

        experience_actions_count = i32::min(max_progress - project_site.progress, base_damage);
        actions_count = i32::min(max_progress - project_site.progress, damage);
    }
    project_site.progress += actions_count;
    project_site.last_crit_outcome = crit_multiplier.ceil() as i32;
    project_site.last_hit_timestamp = ctx.timestamp;

    // Learn about the building and the construction recipe the player helped with
    if !dry_run {
        if let Some(ref b) = building {
            let mut discovery = Discovery::new(actor_id);
            discovery.acquire_building(ctx, b.id);
            discovery.acquire_construction(ctx, project_site.construction_recipe_id);
            discovery.commit(ctx);
        } else if let Some(ref r) = resource {
            let mut discovery = Discovery::new(actor_id);
            discovery.acquire_resource(ctx, r.id);
            discovery.acquire_resource_placement(ctx, project_site.resource_placement_recipe_id);
            discovery.commit(ctx);
        }

        let experience_per_progress = &experience_per_progress[0];
        ExperienceState::add_experience(
            ctx,
            actor_id,
            experience_per_progress.skill_id,
            f32::ceil(experience_per_progress.quantity * experience_actions_count as f32) as i32,
        );

        if actions_count > 0 {
            EquipmentState::try_activate_profession_hit_buffs(ctx, actor_id, skill)?;
        }

        if project_site.progress >= actions_required {
            let mut owner_id = project_site.owner_id;
            if let Some(dimension_network) = DimensionNetworkState::get(ctx, target_coords.dimension) {
                owner_id = dimension_network.claim_entity_id;
            }
            let direction = project_site.direction;
            ctx.db.project_site_state().entity_id().delete(&request.owner_entity_id);

            if let Some(building) = building {
                let entity_id = create_building_component(ctx, owner_id, request.owner_entity_id, direction, &building, actor_id);

                footprint_helpers::update_footprint_after_building_completion(ctx, request.owner_entity_id, direction, &building);

                for (coords, footprint_type) in building.get_footprint(&coordinates, direction) {
                    if footprint_type != FootprintType::Walkable {
                        continue;
                    }

                    let mut footprint = FootprintTileState::get_at_location(ctx, &coords).next().unwrap();
                    footprint.footprint_type = FootprintType::Walkable;

                    ctx.db.footprint_tile_state().entity_id().update(footprint);
                }

                create_building_claim(ctx, request.owner_entity_id, false)?;

                if ctx.db.interior_network_desc().building_id().find(&building.id).is_some() {
                    create_building_interior(ctx, request.owner_entity_id)?;
                }

                BuildingState::create_rental(ctx, request.owner_entity_id, &building, coordinates.dimension)?;
                BuildingState::create_empire_building(
                    ctx,
                    entity_id,
                    &building,
                    actor_id,
                    coordinates,
                    Some(project_site.construction_recipe_id),
                );
                BuildingState::create_waystone(ctx, entity_id, owner_id, &building, coordinates);
                BuildingState::create_bank(ctx, entity_id, owner_id, &building, coordinates)?;
                BuildingState::create_marketplace(ctx, entity_id, owner_id, &building, coordinates)?;
                create_distant_visibile_building(ctx, &building, entity_id, coordinates);
                create_building_spawns(ctx, entity_id);
            } else if let Some(r) = resource {
                ResourceState::spawn_from_construction_site(ctx, request.owner_entity_id, &r, coordinates, direction, r.max_health)?;
            }
            PlayerActionState::success(
                ctx,
                actor_id,
                PlayerActionType::None,
                PlayerActionType::Build.get_layer(ctx),
                0,
                None,
                None,
                request.timestamp,
            );
        } else {
            ctx.db.project_site_state().entity_id().update(project_site);
        }

        PlayerActionState::mark_as_consumed(ctx, actor_id)?;
    }

    Ok(())
}
