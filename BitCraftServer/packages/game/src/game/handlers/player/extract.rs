use bitcraft_macro::feature_gate;
use spacetimedb::rand::Rng;
use std::time::Duration;

use crate::{
    game::{
        claim_helper,
        coordinates::*,
        dimensions,
        discovery::Discovery,
        entities::{buff, building_state::InventoryState, terrain_cell::TerrainCell},
        game_state::{
            self,
            game_state_filters::{self, coordinates_float, has_hitbox_footprint},
        },
        permission_helper,
        reducer_helpers::{
            deployable_helpers::dismount_deployable_and_set_deployable_position, interior_helpers::interior_trigger_collapse,
            player_action_helpers,
        },
        terrain_chunk::TerrainChunkCache,
    },
    messages::{
        action_request::PlayerExtractRequest,
        components::*,
        empire_shared::{empire_chunk_state, empire_player_data_state, EmpirePlayerDataState},
        game_util::{ItemStack, ItemType},
        static_data::*,
    },
    unwrap_or_err,
};

use spacetimedb::{log, ReducerContext, Table};

fn apply_self_buffs(ctx: &ReducerContext, actor_id: u64, self_buffs: &[PlaceableSelfBuffChance]) -> Result<(), String> {
    for self_buff in self_buffs {
        if ctx.rng().gen_range(0.0..=1.0) <= self_buff.chance {
            buff::activate(ctx, actor_id, self_buff.buff_id, self_buff.duration, None)?;
        }
    }

    Ok(())
}

fn format_missing_input_message(ctx: &ReducerContext, required_stack: &ItemStack) -> String {
    let item_name = match required_stack.item_type {
        ItemType::Item => ctx
            .db
            .item_desc()
            .id()
            .find(&required_stack.item_id)
            .map(|item| item.name)
            .unwrap_or_else(|| "Unknown item".into()),
        ItemType::Cargo => ctx
            .db
            .cargo_desc()
            .id()
            .find(&required_stack.item_id)
            .map(|cargo| cargo.name)
            .unwrap_or_else(|| "Unknown cargo".into()),
    };

    format!("Requires {{0}} {{1}}|~{}|~{}", required_stack.quantity, item_name)
}

fn event_delay_recipe_id(ctx: &ReducerContext, request: &PlayerExtractRequest, stats: &CharacterStatsState) -> (Duration, Option<i32>) {
    let recipe = ctx.db.extraction_recipe_desc().id().find(&request.recipe_id);
    if recipe.is_none() {
        return (Duration::ZERO, None);
    }

    let recipe = recipe.unwrap();
    let skill_speed = match recipe.get_skill_type() {
        Some(skill) => stats.get_skill_speed(skill),
        None => 1.0,
    };
    let gather_time_multiplier = 1.0 / (stats.get(CharacterStatType::GatheringSpeed) + skill_speed - 1.0);
    (
        Duration::from_secs_f32(recipe.time_requirement * gather_time_multiplier),
        Some(recipe.id),
    )
}

#[spacetimedb::reducer]
#[feature_gate("extract")]
pub fn extract_start(ctx: &ReducerContext, request: PlayerExtractRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");

    let target = Some(request.target_entity_id);
    let (delay, recipe_id) = event_delay_recipe_id(ctx, &request, &stats);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Extract,
        target,
        recipe_id,
        delay,
        reduce(ctx, actor_id, request, stats, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("extract")]
pub fn extract(ctx: &ReducerContext, request: PlayerExtractRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Extract.get_layer(ctx),
        reduce(ctx, actor_id, request, stats, false),
    )
}

fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    request: PlayerExtractRequest,
    stats: CharacterStatsState,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::Extract, request.timestamp)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Extract, Some(request.target_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Extract, request.timestamp)?;
    }

    let mut terrain_cache = TerrainChunkCache::empty();

    let recipe = unwrap_or_err!(ctx.db.extraction_recipe_desc().id().find(&request.recipe_id), "Recipe not found.");

    let stamina_state = unwrap_or_err!(
        ctx.db.stamina_state().entity_id().find(&actor_id),
        "Player missing stamina component!"
    );

    if stamina_state.stamina < recipe.stamina_requirement as f32 {
        return Err("Not enough stamina!".into());
    }

    let deposit_entity_id;

    let deposit = if recipe.cargo_id == 0 {
        // An empty deposit is a "normal" reaction if you spam to extract or if multiple players work on the same node.
        // Therefore, a normal termination would be better than an error message.
        if let Some(d) = ctx.db.resource_state().entity_id().find(&request.target_entity_id) {
            if ResourceHealthState::is_depleted(ctx, d.entity_id) {
                // This might happen for animated depleted resources
                return Err("Deposit already depleted.".into());
            }

            if d.resource_id != recipe.resource_id {
                return Err("Invalid operation".into());
            }

            deposit_entity_id = d.entity_id;
            d
        } else {
            return Err("Deposit already depleted.".into());
        }
    } else {
        log::error!("OBSOLETE - YOU SHOULDN'T BE ABLE TO EXTRACT CARGOS ANYMORE");
        return Err("OBSOLETE - YOU SHOULDN'T BE ABLE TO EXTRACT CARGOS ANYMORE".into());
    };

    // Contribution (crumb trail prizes) gating
    if let Some(contribution_lock) = ctx.db.crumb_trail_contribution_lock_state().entity_id().find(deposit_entity_id) {
        if let Some(mut prospecting) = ctx.db.prospecting_state().entity_id().find(actor_id) {
            if prospecting.crumb_trail_entity_id != contribution_lock.crumb_trail_entity_id {
                return Err("You must have helped find this resource to be able to gather from it".into());
            } else {
                if !dry_run {
                    // jump to final step
                    prospecting.ongoing_step = prospecting.total_steps - 1;

                    prospecting.contribution -= 1;
                    if ctx
                        .db
                        .crumb_trail_contribution_spent_state()
                        .player_and_crumb_entity_id()
                        .filter((actor_id, contribution_lock.crumb_trail_entity_id))
                        .next()
                        .is_none()
                    {
                        ctx.db
                            .crumb_trail_contribution_spent_state()
                            .insert(CrumbTrailContributionSpentState {
                                entity_id: game_state::create_entity(ctx),
                                player_entity_id: actor_id,
                                crumb_trail_entity_id: contribution_lock.crumb_trail_entity_id,
                            });
                    }
                    if prospecting.contribution <= 0 {
                        ctx.db.prospecting_state().entity_id().delete(prospecting.entity_id);
                    } else {
                        ctx.db.prospecting_state().entity_id().update(prospecting);
                    }
                }
            }
        } else {
            if ctx
                .db
                .crumb_trail_contribution_spent_state()
                .player_and_crumb_entity_id()
                .filter((actor_id, contribution_lock.crumb_trail_entity_id))
                .next()
                .is_some()
            {
                return Err("You've already gathered all you can from this".into());
            }
            return Err("You must have helped find this resource to be able to gather from it".into());
        }
    }

    let coordinates = game_state_filters::coordinates(ctx, request.target_entity_id);
    let actor_coords: FloatHexTile = coordinates_float(ctx, actor_id);

    let mut target_coords = SmallHexTile::from(coordinates);
    // if resource has a footprint, find the closest footprint tile
    if let Some(resource) = ctx.db.resource_desc().id().find(&deposit.resource_id) {
        let footprint = resource.get_footprint(&coordinates.into(), deposit.direction_index);
        if footprint.len() > 0 {
            let mut min_dist = actor_coords.distance_to(target_coords.into());
            for (coord, footprint_type) in footprint {
                if footprint_type != FootprintType::Perimeter {
                    let dist = actor_coords.distance_to(coord.into());
                    if dist < min_dist {
                        target_coords = coord.into();
                        min_dist = dist;
                    }
                }
            }
        }
    }

    let terrain_source = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &actor_coords.parent_large_tile()),
        "Invalid location"
    );

    let mounting = ctx.db.mounting_state().entity_id().find(&actor_id);

    // If a player isn't on a deployable and is in swim-deep water they aren't allowed to extract
    if terrain_source.player_should_swim() && mounting.is_none() {
        return Err("Action disallowed while swimming".into());
    }

    let mut deployable_radius = 0.0;
    if let Some(mounting) = mounting {
        let deployable = ctx
            .db
            .deployable_state_v2()
            .entity_id()
            .find(mounting.deployable_entity_id)
            .unwrap();
        let deployable_desc = ctx.db.deployable_desc().id().find(deployable.deployable_description_id).unwrap();
        deployable_radius = deployable_desc.radius;
        if !deployable_desc.allow_driver_extract {
            dismount_deployable_and_set_deployable_position(ctx, actor_id, false, actor_coords.into());
        }
    }

    if actor_coords.distance_to(target_coords.into()) > recipe.range as f32 + deployable_radius + 1.0 {
        return Err("You are too far.".into());
    }

    //DAB Note: re-enable and debug after 5.0
    //reducer_helpers::validate_action_elevation(&mut terrain_cache, actor_coords, target_coords, true, 2, "extract")?;
    // validate claims

    if !PermissionState::can_interact_with_tile(ctx, actor_id, coordinates.into(), Permission::Usage) {
        return Err("You don't have permission to forage on this claim.".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, coordinates.into(), actor_id, ClaimPermission::Usage) {
        return Err("You don't have permission to forage on this claim.".into());
    }

    if request.clear_from_claim && claim_helper::get_claim_on_tile(ctx, coordinates).is_none() {
        return Err("You can only clear a resource on a claim".into());
    }

    // Validate Tool Requirement
    let skill_power = match recipe.get_skill_type() {
        Some(skill) => stats.get_skill_power(skill),
        None => 0.0,
    };

    let mut tool_power = 1.0;

    if request.clear_from_claim {
        // demolish on claim gets the equipped tool power no matter whether it fills requirements or not
        if recipe.tool_requirements.len() > 0 {
            tool_power = match ToolDesc::get_equipped_tool(ctx, actor_id, &recipe.tool_requirements[0]) {
                Some(t) => t.power as f32,
                None => 1.0,
            };
        }
    } else {
        if recipe.tool_requirements.len() > 0 {
            tool_power = match ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0]) {
                Ok(t) => t.power as f32,
                Err(s) => {
                    if recipe.allow_use_hands {
                        1.0
                    } else {
                        return Err(s.into());
                    }
                }
            };
        }
    }
    tool_power += skill_power;

    // Validate Skill requirement
    let player_level = ctx
        .db
        .experience_state()
        .entity_id()
        .find(&actor_id)
        .unwrap()
        .get_level(recipe.level_requirements[0].skill_id);
    let deposit_desired_level = recipe.level_requirements[0].level;

    // Validate empire requirements
    if let Some(empire_permission) = recipe.empire_permission_required {
        let chunk_index = coordinates.chunk_coordinates().chunk_index();
        if let Some(empire_chunk) = ctx.db.empire_chunk_state().chunk_index().find(chunk_index) {
            if let Some(player_empire) = ctx.db.empire_player_data_state().entity_id().find(actor_id) {
                if empire_chunk.empire_entity_id != player_empire.empire_entity_id {
                    return Err("Cannot extract this resource when not fully under your empire control".into());
                }
                if !EmpirePlayerDataState::has_permission(ctx, actor_id, empire_permission) {
                    return Err("You don't have the permissions to extract this".into());
                }
            } else {
                return Err("You need to be part of an empire to extract this".into());
            }
        } else {
            return Err("This resource is not controlled by your empire".into());
        }
    }

    for required_knowledge_id in &recipe.required_knowledges {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *required_knowledge_id) {
            return Err("You don't have the knowledge required to perform this action".into());
        }
    }

    if recipe.blocking_knowledges.len() > 0 {
        let mut possess_all_knowledges = true;
        let secondary_knowledge = ctx.db.knowledge_secondary_state().entity_id().find(actor_id).unwrap();
        for knowledge_id in &recipe.blocking_knowledges {
            possess_all_knowledges &= secondary_knowledge
                .entries
                .iter()
                .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired);
        }
        if possess_all_knowledges {
            return Err("You don't need this resource anymore".into());
        }
    }

    // validation is complete
    if !dry_run {
        if !StaminaState::decrease_stamina(ctx, actor_id, recipe.stamina_requirement as f32) {
            return Err("Failed to update stamina".into());
        }

        if !recipe.consumed_item_stacks.is_empty() && !request.clear_from_claim {
            let consumed_item_stacks = recipe
                .consumed_item_stacks
                .iter()
                .filter_map(|stack| {
                    let rnd = ctx.rng().gen_range(0.0..=1.0);
                    if rnd <= stack.consumption_chance {
                        Some(ItemStack::from(ctx, stack))
                    } else {
                        None
                    }
                })
                .collect();

            if let Err(err) =
                InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &consumed_item_stacks, |x| {
                    get_distance(ctx, &deposit, coordinates, x)
                })
            {
                if consumed_item_stacks.is_empty() {
                    return Err(err);
                }

                return Err(format_missing_input_message(ctx, &consumed_item_stacks[0]));
            }
        }

        if recipe.tool_durability_lost > 0 {
            InventoryState::reduce_tool_durability(ctx, actor_id, recipe.tool_requirements[0].tool_type, recipe.tool_durability_lost);
        }

        let resource = ctx.db.resource_desc().id().find(&deposit.resource_id).unwrap();
        let mut deposit_health = ctx.db.resource_health_state().entity_id().find(&deposit_entity_id).unwrap();
        if !resource.ignore_damage {
            // Static data may lower max health while this resource still has its old health.
            // Clamp first so this extraction's damage is applied to the new effective health.
            deposit_health.health = i32::clamp(deposit_health.health, 0, resource.max_health);
        }

        // Check Extract Yield (factoring power of tool since it affect amount yielded)
        // Get crit rate
        let (success, is_crit, damage_output, experience_damage_output) = if recipe.cargo_id != 0 {
            (true, false, 1, 0)
        } else {
            if request.clear_from_claim || player_level >= deposit_desired_level {
                let crit_multiplier = stats.get_final_crit_multiplier(ctx, recipe.get_skill_type());
                let base_damage = tool_power.round() as i32;
                let damage = (tool_power * crit_multiplier).round() as i32;

                let experience_damage_output = i32::min(deposit_health.health, base_damage);
                let damage_output = i32::min(deposit_health.health, damage);
                (
                    true,
                    crit_multiplier > 1.0,
                    damage_output,
                    if request.clear_from_claim { 0 } else { experience_damage_output },
                )
            } else {
                (false, false, 0, 0)
            }
        };

        let mut discovery = Discovery::new(actor_id);
        let mut output = Vec::new();

        if success {
            if !request.clear_from_claim {
                // demolish on claim doesn't yield output
                for stack in &recipe.extracted_item_stacks {
                    if let Some(rolled) = stack.roll(ctx, damage_output) {
                        output.push(rolled);
                    }
                }

                // Quest drops
                if let Some(mut drops) = QuestDropDesc::roll_extraction(ctx, actor_id, request.recipe_id, damage_output) {
                    output.append(&mut drops);
                }
            }

            let experience_per_progress = if request.clear_from_claim {
                // demolish on claim grants no experience
                None
            } else {
                recipe.experience_per_progress.get(0)
            };

            // assign completion experience proportional to damage done / max health
            let fractional_progress = experience_damage_output as f32;
            if fractional_progress > 0.0 {
                if let Some(experience_per_progress) = experience_per_progress {
                    ExperienceState::add_experience(
                        ctx,
                        actor_id,
                        experience_per_progress.skill_id,
                        f32::ceil(experience_per_progress.quantity * fractional_progress) as i32,
                    );
                }
            }

            if !request.clear_from_claim {
                // demolish on claim doesn't provide discovery
                discovery.acquire_extract(ctx, recipe.id);
                discovery.acquire_resource(ctx, recipe.resource_id);
            }

            if !request.clear_from_claim && damage_output > 0 {
                if let Some(spawned_placeables) = &recipe.spawned_placeables {
                    for spawned_placeable in spawned_placeables {
                        let scaled_spawn_chance = (spawned_placeable.chance * damage_output as f32).clamp(0.0, 1.0);
                        if scaled_spawn_chance > 0.0
                            && (scaled_spawn_chance >= 1.0 || ctx.rng().gen_range(0.0..=1.0) <= scaled_spawn_chance)
                        {
                            let _ = PlaceableState::spawn_in_radius_band(
                                ctx,
                                spawned_placeable.placeable_id,
                                actor_id,
                                coordinates,
                                deposit.direction_index,
                                spawned_placeable.radius_min,
                                spawned_placeable.radius_max,
                            );
                        }
                    }
                }

                if !request.clear_from_claim {
                    if let Some(self_buffs) = &recipe.self_buffs {
                        apply_self_buffs(ctx, actor_id, self_buffs)?;
                    }
                }

                EquipmentState::try_activate_profession_hit_buffs(ctx, actor_id, recipe.get_skill_type())?;
            }
        }

        let mut extract_outcome: ExtractOutcomeStateV2 = ctx.db.extract_outcome_state().entity_id().find(&actor_id).unwrap();
        extract_outcome.target_entity_id = deposit_entity_id;
        extract_outcome.damage = damage_output;
        extract_outcome.last_timestamp = ctx.timestamp;
        extract_outcome.is_crit = is_crit;
        ctx.db.extract_outcome_state().entity_id().update(extract_outcome);

        if !resource.ignore_damage {
            deposit_health.health = i32::max(deposit_health.health - damage_output, 0);

            if deposit_health.health <= 0 {
                // Give end of resource items (unless demolishing on claim)
                if !resource.on_destroy_yield.is_empty() && !request.clear_from_claim {
                    for stack in resource.on_destroy_yield {
                        if stack.item_type == ItemType::Cargo {
                            continue;
                        }

                        output.push(stack);
                    }
                }

                if let Some(collapse_trigger) = ctx.db.interior_collapse_trigger_state().entity_id().find(&deposit_entity_id) {
                    interior_trigger_collapse(ctx, collapse_trigger.dimension_network_entity_id)?;
                }

                // delete the deposit along its footprints and location then spawn the replacement resource if needed
                let resource_id = deposit.resource_id;
                let resource_direction = deposit.direction_index;
                deposit.despawn_self(ctx);
                ResourceState::produce_offspawn(ctx, resource_id, coordinates, resource_direction);
                PlayerActionState::success(
                    ctx,
                    actor_id,
                    PlayerActionType::None,
                    PlayerActionType::Extract.get_layer(ctx),
                    0,
                    None,
                    None,
                    request.timestamp,
                );
            } else {
                ctx.db.resource_health_state().entity_id().update(deposit_health);
            }
        }

        discovery.commit(ctx);

        InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            actor_id,
            &output,
            |x| get_distance(ctx, &deposit, coordinates, x),
            true,
            || {
                find_valid_cargo_coords(
                    ctx,
                    &mut terrain_cache,
                    coordinates.into(),
                    deposit.resource_id,
                    deposit.direction_index,
                )
            },
            false,
        )?;

        PlayerActionState::mark_as_consumed(ctx, actor_id)?;
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

fn get_distance(ctx: &ReducerContext, deposit: &ResourceState, center: SmallHexTile, coordinates: SmallHexTile) -> i32 {
    coordinates.distance_to_footprint(deposit.footprint(ctx, center))
}

fn find_valid_cargo_coords(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    source_coord: SmallHexTile,
    deposit_resource_id: i32,
    deposit_direction: i32,
) -> Vec<SmallHexTile> {
    let mut spawn_coordinates = Vec::new();
    if let Some(terrain_source) = terrain_cache.get_terrain_cell(ctx, &source_coord.parent_large_tile()) {
        // Deposit or cargo surrounding positions by default
        for coord in SmallHexTile::coordinates_in_radius(source_coord, 1) {
            spawn_coordinates.push(coord);
        }

        // if resource has a footprint, add the surround on each hitbox footprint
        if let Some(resource) = ctx.db.resource_desc().id().find(&deposit_resource_id) {
            let footprint = resource.get_footprint(&source_coord, deposit_direction);
            if footprint.len() > 0 {
                for (coord, footprint_type) in footprint {
                    if footprint_type == FootprintType::Hitbox {
                        for surround in SmallHexTile::coordinates_in_radius(coord, 1) {
                            spawn_coordinates.push(surround);
                        }
                    }
                }
            }
        }

        spawn_coordinates.sort_by(|a, b| a.hashcode().cmp(&b.hashcode()));
        spawn_coordinates.dedup();
        spawn_coordinates = spawn_coordinates
            .iter()
            .filter(|coord| validate_tile(ctx, terrain_cache, **coord, &terrain_source, 2))
            .map(|c| *c)
            .collect();
    }
    spawn_coordinates
}

fn validate_tile(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    target_coordinates: SmallHexTile,
    terrain_source: &TerrainCell,
    max_elevation: i16,
) -> bool {
    // Don't spawn onto "hitbox" footprints
    if has_hitbox_footprint(ctx, target_coordinates) {
        return false;
    }

    // find terrain
    if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &target_coordinates.parent_large_tile()) {
        // Prevent movement over elevation (unless in water - todo if we have waterfalls)
        let elevation_diff = i16::abs(terrain_source.elevation - terrain_target.elevation);
        if elevation_diff > max_elevation {
            // Don't spawn in a hole or over a cliff
            return false;
        }

        if terrain_source.is_submerged() != terrain_target.is_submerged() {
            // Don't spawn in or outside water (depending on the source)
            return false;
        }

        if target_coordinates.dimension != dimensions::OVERWORLD && !game_state_filters::is_interior_tile_walkable(ctx, target_coordinates)
        {
            return false;
        }
    } else {
        // No terrain, spawning outside the map?
        return false;
    }
    true
}
