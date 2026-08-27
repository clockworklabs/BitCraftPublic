use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        claim_helper,
        discovery::Discovery,
        entities::building_state::BuildingState,
        game_state::{self, game_state_filters},
        reducer_helpers::{deployable_helpers::dismount_deployable_and_set_deployable_position, player_action_helpers},
    },
    messages::{
        action_request::{PlayerCraftContinueRequest, PlayerCraftInitiateRequest},
        components::*,
        game_util::ItemStack,
        static_data::*,
    },
    unwrap_or_err,
};

pub fn event_delay(recipe: &CraftingRecipeDesc, stats: &CharacterStatsState) -> Duration {
    let skill_speed = match recipe.get_skill_type() {
        Some(skill) => stats.get_skill_speed(skill),
        None => 1.0,
    };
    let craft_time_multiplier = 1.0 / (stats.get(CharacterStatType::CraftingSpeed) + skill_speed - 1.0);
    return Duration::from_secs_f32(recipe.time_requirement * craft_time_multiplier);
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_initiate_start(ctx: &ReducerContext, request: PlayerCraftInitiateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let recipe = unwrap_or_err!(ctx.db.crafting_recipe_desc().id().find(&request.recipe_id), "Invalid recipe");
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&request.building_entity_id),
        "Building doesn't exist"
    );

    if request.count > 999999 {
        return Err("Quantity too large".into());
    }

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building.building_description_id),
        "Building description doesn't exist"
    );

    if let Some(ref building_requirement) = recipe.building_requirement {
        if !building_desc.fulfills_function(building_requirement.building_type, building_requirement.tier) {
            return Err("Invalid building".into());
        }
    }

    building.ensure_claim_tech(ctx)?;

    validate_slots(ctx, actor_id, &building, false)?;

    let target = Some(request.building_entity_id);

    // Create a new progressive action for this building
    let progressive_action =
        ProgressiveActionState::new(ctx, actor_id, building.entity_id, &building_desc, request.recipe_id, request.count);
    ctx.db.progressive_action_state().try_insert(progressive_action.clone())?;

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Craft,
        target,
        Some(recipe.id),
        event_delay(&recipe, &stats),
        reduce(
            ctx,
            actor_id,
            request.recipe_id,
            request.building_entity_id,
            progressive_action,
            stats,
            Some(request.count),
            request.timestamp,
            request.is_public,
            true,
        ),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_initiate(ctx: &ReducerContext, request: PlayerCraftInitiateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    // This requires some out-of-the-box thinking. The previous successful craft_initiate_start assigns the action data,
    // including the owner_entity_id being set to the player_id and the expiration timestamp being set.
    // Since we don't know the action's entity_id when we queue the outcome, we need to find the pocket that was assigned by the player
    // with the most recent expiration timestamp. Filtering by building might not be necessary, but we will leave it in case different buildings have different lock times in the future.

    let progressive_action = unwrap_or_err!(
        ctx.db
            .progressive_action_state()
            .building_entity_id()
            .filter(request.building_entity_id)
            .filter(|a| a.owner_entity_id == actor_id)
            .max_by_key(|a| a.lock_expiration),
        "No crafting in progress"
    );
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");

    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Craft.get_layer(ctx),
        reduce(
            ctx,
            actor_id,
            progressive_action.recipe_id,
            progressive_action.building_entity_id,
            progressive_action,
            stats,
            None,
            request.timestamp,
            request.is_public,
            false,
        ),
    )
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_continue_start(ctx: &ReducerContext, request: PlayerCraftContinueRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let progressive_action = unwrap_or_err!(
        ctx.db
            .progressive_action_state()
            .entity_id()
            .find(&request.progressive_action_entity_id),
        "Craft no longer exists"
    );
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");

    let recipe = unwrap_or_err!(
        ctx.db.crafting_recipe_desc().id().find(&progressive_action.recipe_id),
        "Invalid recipe"
    );
    let delay = event_delay(&recipe, &stats);

    let target = Some(progressive_action.building_entity_id);

    if progressive_action.get_status_from_recipe(&recipe, ctx.timestamp) == ProgressiveActionStatus::Suspended {
        let building = unwrap_or_err!(
            ctx.db.building_state().entity_id().find(&progressive_action.building_entity_id),
            "Building doesn't exist"
        );

        validate_slots(ctx, actor_id, &building, true)?;
    }

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Craft,
        target,
        Some(recipe.id),
        delay,
        reduce(
            ctx,
            actor_id,
            progressive_action.recipe_id,
            progressive_action.building_entity_id,
            progressive_action,
            stats,
            None,
            request.timestamp,
            false,
            true,
        ),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_continue(ctx: &ReducerContext, request: PlayerCraftContinueRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let progressive_action = unwrap_or_err!(
        ctx.db
            .progressive_action_state()
            .entity_id()
            .find(&request.progressive_action_entity_id),
        "Craft no longer exists"
    );
    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");

    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Craft.get_layer(ctx),
        reduce(
            ctx,
            actor_id,
            progressive_action.recipe_id,
            progressive_action.building_entity_id,
            progressive_action,
            stats,
            None,
            request.timestamp,
            false,
            false,
        ),
    )
}

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    recipe_id: i32,
    building_entity_id: u64,
    mut progressive_action: ProgressiveActionState,
    stats: CharacterStatsState,
    new_craft_count: Option<i32>,
    timestamp: u64,
    is_public: bool,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if new_craft_count.is_some() && new_craft_count.unwrap() <= 0 {
        return Err("Invalid quantity".into());
    }

    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::Craft, timestamp)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Craft, Some(building_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Craft, timestamp)?;
    }

    let recipe = unwrap_or_err!(ctx.db.crafting_recipe_desc().id().find(&recipe_id), "Invalid recipe");

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
            return Err("You already know everything this recipe has to offer".into());
        }
    }

    // TODO: Remove after replacing passive crafting with refining
    let is_passive = recipe.is_passive && recipe.building_requirement.is_some();
    if is_passive {
        return Err("This handler is for active crafting only".into());
    }

    let player_coord = game_state_filters::coordinates_float(ctx, actor_id);

    if ctx.db.mounting_state().entity_id().find(&actor_id).is_some() {
        dismount_deployable_and_set_deployable_position(ctx, actor_id, false, player_coord.into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&progressive_action.building_entity_id),
        "Invalid building"
    );

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Usage) {
        return Err("You don't have the permission to use this building".into());
    }

    if progressive_action.owner_entity_id != actor_id
        && ctx
            .db
            .public_progressive_action_state()
            .entity_id()
            .find(progressive_action.entity_id)
            .is_none()
    {
        return Err("You don't own this craft".into());
    }

    let craft_status = progressive_action.get_status_from_recipe(&recipe, ctx.timestamp);
    if craft_status == ProgressiveActionStatus::Completed {
        return Err("Crafting is already complete".into());
    }

    if craft_status == ProgressiveActionStatus::Suspended {}

    let stamina_state = unwrap_or_err!(
        ctx.db.stamina_state().entity_id().find(&actor_id),
        "Player missing stamina component!"
    );
    if stamina_state.stamina < recipe.stamina_requirement as f32 {
        return Err("Not enough stamina.".into());
    }

    if !dry_run {
        StaminaState::add_player_stamina(ctx, actor_id, -recipe.stamina_requirement as f32);
    }

    if recipe.required_claim_tech_id != 0 {
        // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
        let claim_tech = unwrap_or_err!(
            ctx.db.claim_tech_state().entity_id().find(&building.claim_entity_id),
            "You require a claim to craft this recipe"
        );
        if !claim_tech.has_unlocked_tech(recipe.required_claim_tech_id) {
            return Err("Missing claim upgrades".into());
        }
    }

    // Make sure the player has all the required knowledges to complete the action
    // (is_public is only used to initiate craft, in which case we don't need to validate the knowledge)
    if !is_public {
        for required_knowledge_id in &recipe.required_knowledges {
            if !Discovery::already_acquired_secondary(ctx, actor_id, *required_knowledge_id) {
                return Err("You don't have the knowledge required to craft this".into());
            }
        }
    }

    // if player is not inside a building check if the request is for
    // an unenterable building (e.g. firepit) and if they are close enough
    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building.building_description_id),
        "Unknown building type"
    );

    if building_desc.unenterable {
        // Temporary: allow a distance of 2 for when you right-click on building while moving and end up 1 tile too far by completing the current move
        if building.distance_to(ctx, &player_coord.into()) > 2 {
            return Err("Too far".into());
        }
    } else {
        let player_building =
            game_state_filters::building_at_coordinates(ctx, &game_state_filters::coordinates_float(ctx, actor_id).into());
        match player_building {
            Some(player_building) => {
                if player_building.entity_id != progressive_action.building_entity_id {
                    return Err("Player is inside another building".into());
                }
            }
            None => return Err("Player isn't inside a building".into()),
        }
    }

    // Validate Tool Requirement
    let mut tool: Option<ToolDesc> = None;
    let mut meets_tool_requirements = true;

    if recipe.tool_requirements.len() > 0 {
        tool = match ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0]) {
            Ok(t) => Some(t),
            Err(s) => {
                if recipe.allow_use_hands {
                    None
                } else {
                    meets_tool_requirements = false;

                    if progressive_action.owner_entity_id != actor_id {
                        return Err(s.into());
                    } else {
                        None
                    }
                }
            }
        };
    }

    // Limit to 1 lock per building per player if not a member of the claim, but only when working on our own craft
    if progressive_action.owner_entity_id == actor_id {
        if building.claim_entity_id != 0 {
            if ProgressiveActionState::get_active_locks_on_building(ctx, actor_id, building.entity_id)
                .any(|action| action.entity_id != progressive_action.entity_id)
            {
                if let Some(claim) = ctx.db.claim_state().entity_id().find(&building.claim_entity_id) {
                    if claim.owner_player_entity_id != 0 {
                        if !claim.get_member(ctx, actor_id).is_some() {
                            return Err("Non claim members can only have 1 ongoing craft per building".into());
                        }
                    } else {
                        // If there is no owner, this must be a neutral claim
                        return Err("You can only have 1 ongoing craft per shared building".into());
                    }
                } else {
                    return Err("Invalid claim".into());
                }
            }
        }
    }

    // New Craft (will be only set once in player_craft_initiate_start)
    if let Some(count) = new_craft_count {
        progressive_action.craft_count = count;
        progressive_action.recipe_id = recipe_id;

        let consumed_stacks: Vec<ItemStack> = recipe
            .consumed_item_stacks
            .iter()
            .map(|is| {
                Ok(ItemStack::new(
                    ctx,
                    is.item_id,
                    is.item_type,
                    is.quantity.checked_mul(count).ok_or_else(|| "Quantity too large".to_string())?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;

        InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &consumed_stacks, |x| {
            building.distance_to(ctx, &x)
        })?;
    }

    //check player level
    let player_level = ctx
        .db
        .experience_state()
        .entity_id()
        .find(&actor_id)
        .unwrap()
        .get_level(recipe.level_requirements[0].skill_id);
    let recipe_desired_skill_level = recipe.level_requirements[0].level;
    let meets_level_requirements = player_level >= recipe_desired_skill_level;

    if !meets_level_requirements && progressive_action.owner_entity_id != actor_id {
        return Err("Your skill level is too low.".into());
    }

    progressive_action.set_expiration(ctx);
    progressive_action.preparation = dry_run;

    if meets_tool_requirements && meets_level_requirements {
        if !dry_run {
            let skill = recipe.get_skill_type();

            let mut actions_count = 0;
            let mut crit_multiplier = 1.0;

            if player_level >= recipe_desired_skill_level {
                let skill_power = match skill {
                    Some(s) => stats.get_skill_power(s),
                    None => 0.0,
                };
                let tool_power = if let Some(tool) = tool { tool.power as f32 } else { 1.0 } + skill_power;

                crit_multiplier = stats.get_final_crit_multiplier(ctx, skill);

                let base_damage = tool_power.round() as i32;
                let damage = (tool_power * crit_multiplier).round() as i32;

                let experience_actions_count = i32::min(
                    recipe.actions_required * progressive_action.craft_count - progressive_action.progress,
                    base_damage,
                );
                actions_count = i32::min(
                    recipe.actions_required * progressive_action.craft_count - progressive_action.progress,
                    damage,
                );

                let experience_per_progress = recipe.experience_per_progress.get(0);
                if let Some(experience_per_progress) = experience_per_progress {
                    let quantity = f32::ceil(experience_per_progress.quantity * experience_actions_count as f32) as i32;
                    ExperienceState::add_experience(ctx, actor_id, experience_per_progress.skill_id, quantity);

                    if building.claim_entity_id != 0 {
                        claim_helper::mint_hex_coins(ctx, building.claim_entity_id, quantity as u32);
                    }
                }

                if actions_count > 0 {
                    EquipmentState::try_activate_profession_hit_buffs(ctx, actor_id, skill)?;
                }
            }
            progressive_action.last_crit_outcome = crit_multiplier.ceil() as i32;
            progressive_action.progress += actions_count;
            if progressive_action.progress >= recipe.actions_required * progressive_action.craft_count {
                PlayerActionState::success(
                    ctx,
                    actor_id,
                    PlayerActionType::None,
                    PlayerActionType::Craft.get_layer(ctx),
                    0,
                    None,
                    None,
                    timestamp,
                );
            }

            if recipe.tool_durability_lost > 0 {
                InventoryState::reduce_tool_durability(ctx, actor_id, recipe.tool_requirements[0].tool_type, recipe.tool_durability_lost);
            }

            PlayerActionState::mark_as_consumed(ctx, actor_id)?;
        }
    } else {
        PlayerActionState::clear_by_entity_id(ctx, actor_id)?;
    }

    if is_public {
        ctx.db.public_progressive_action_state().insert(PublicProgressiveActionState {
            entity_id: progressive_action.entity_id,
            building_entity_id: progressive_action.building_entity_id,
            owner_entity_id: progressive_action.owner_entity_id,
        });
    }

    //We WANT to update these tables on start request to make sure player locks the pocket
    ctx.db.progressive_action_state().entity_id().update(progressive_action);

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

fn validate_slots(ctx: &ReducerContext, actor_id: u64, building: &BuildingState, resume: bool) -> Result<(), String> {
    // Only resume/start if there is at least one active slot free in the building and the concurrent slots aren't all occupied
    let mut active_count = 0;
    let mut player_count = 0;
    for action in ctx.db.progressive_action_state().building_entity_id().filter(building.entity_id) {
        if action.get_status(ctx) == ProgressiveActionStatus::Active {
            active_count += 1;
        }
        if action.owner_entity_id == actor_id {
            player_count += 1;
        }
    }

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building.building_description_id),
        "Building description doesn't exist"
    );

    let max_count = building_desc.crafting_slots();
    if active_count >= max_count {
        return Err("Every crafting slot of this building is busy at the moment. Try again later.".into());
    }

    let max_concurrent = building_desc.concurrent_slots();
    if player_count > max_concurrent {
        return Err("Collect or complete existing crafts first.".into());
    }
    if player_count == max_concurrent && !resume {
        return Err("Collect or complete existing crafts before starting a new one.".into());
    }
    Ok(())
}
