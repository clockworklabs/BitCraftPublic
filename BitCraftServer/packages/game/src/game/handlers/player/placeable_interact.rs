use std::time::Duration;

use bitcraft_macro::feature_gate;
use spacetimedb::rand::Rng;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        discovery::Discovery,
        entities::buff,
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
        terrain_chunk::TerrainChunkCache,
    },
    messages::{
        action_request::PlayerPlaceableInteractRequest,
        components::*,
        game_util::{ItemStack, ItemType},
        static_data::{PlaceableSelfBuffChance, *},
    },
    unwrap_or_err, InventoryState,
};

fn apply_self_buffs(ctx: &ReducerContext, actor_id: u64, self_buffs: &[PlaceableSelfBuffChance]) -> Result<(), String> {
    for self_buff in self_buffs {
        let roll = ctx.rng().gen_range(0.0..=1.0);
        if roll <= self_buff.chance {
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

fn event_delay_recipe_id(
    ctx: &ReducerContext,
    request: &PlayerPlaceableInteractRequest,
    stats: &CharacterStatsState,
) -> (Duration, Option<i32>) {
    let recipe = ctx.db.placeable_interaction_desc().id().find(&request.interaction_id);
    if let Some(recipe) = recipe {
        let skill_speed = match recipe.get_skill_type() {
            Some(skill) => stats.get_skill_speed(skill),
            None => 1.0,
        };
        let gathering_speed = stats.get(CharacterStatType::GatheringSpeed).max(1.0);
        let time_multiplier = 1.0 / (gathering_speed + skill_speed - 1.0).max(0.01);
        return (Duration::from_secs_f32(recipe.time_requirement * time_multiplier), Some(recipe.id));
    }

    (Duration::ZERO, None)
}

#[spacetimedb::reducer]
#[feature_gate("extract")]
pub fn placeable_interact_start(ctx: &ReducerContext, request: PlayerPlaceableInteractRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    let target = Some(request.target_entity_id);
    let (delay, recipe_id) = event_delay_recipe_id(ctx, &request, &stats);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::InteractPlaceable,
        target,
        recipe_id,
        delay,
        reduce(ctx, actor_id, request, stats, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("extract")]
pub fn placeable_interact(ctx: &ReducerContext, request: PlayerPlaceableInteractRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::InteractPlaceable.get_layer(ctx),
        reduce(ctx, actor_id, request, stats, false),
    )
}

fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    request: PlayerPlaceableInteractRequest,
    stats: CharacterStatsState,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::InteractPlaceable, request.timestamp)?;
    if !dry_run {
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::InteractPlaceable, Some(request.target_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::InteractPlaceable, request.timestamp)?;
    }

    let placeable = unwrap_or_err!(
        ctx.db.placeable_state().entity_id().find(&request.target_entity_id),
        "That placeable no longer exists"
    );
    if placeable.owner_entity_id != actor_id {
        return Err("You don't have permission to interact with this placeable".into());
    }

    let recipe = unwrap_or_err!(
        ctx.db.placeable_interaction_desc().id().find(&request.interaction_id),
        "Unknown placeable interaction"
    );
    if placeable.placeable_id != recipe.placeable_id {
        return Err("Invalid operation".into());
    }

    let actor_coords = game_state_filters::coordinates_float(ctx, actor_id);
    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_source = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &actor_coords.parent_large_tile()),
        "Invalid location"
    );
    if terrain_source.player_should_swim() && ctx.db.mounting_state().entity_id().find(&actor_id).is_none() {
        return Err("Cannot interact while swimming".into());
    }

    let placeable_location = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(&placeable.entity_id),
        "Placeable is missing a location"
    );
    let placeable_coordinates = placeable_location.coordinates();

    // include 1 extra range as tolerance
    if actor_coords.distance_to(placeable_coordinates.into()) > recipe.range as f32 + 1.0 {
        return Err("You are too far.".into());
    }

    validate_knowledges(ctx, actor_id, &recipe.required_knowledges, &recipe.blocking_knowledges)?;

    let stamina_state = unwrap_or_err!(
        ctx.db.stamina_state().entity_id().find(&actor_id),
        "Player missing stamina component!"
    );
    if stamina_state.stamina < recipe.stamina_requirement {
        return Err("Not enough stamina!".into());
    }

    let mut tool_power = 1.0;
    let mut used_tool_type = None;
    if !recipe.tool_requirements.is_empty() {
        match ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0]) {
            Ok(tool) => {
                tool_power = tool.power as f32;
                used_tool_type = Some(recipe.tool_requirements[0].tool_type);
            }
            Err(err) => {
                if recipe.allow_use_hands {
                    tool_power = 1.0;
                } else {
                    return Err(err);
                }
            }
        }
    }

    let skill_power = match recipe.get_skill_type() {
        Some(skill) => stats.get_skill_power(skill),
        None => 0.0,
    };
    tool_power += skill_power;

    let meets_level_requirements = if recipe.level_requirements.is_empty() {
        true
    } else {
        let experience = unwrap_or_err!(
            ctx.db.experience_state().entity_id().find(&actor_id),
            "Player has no experience state"
        );
        recipe
            .level_requirements
            .iter()
            .all(|requirement| experience.get_level(requirement.skill_id) >= requirement.level)
    };

    if !dry_run {
        if !StaminaState::decrease_stamina(ctx, actor_id, recipe.stamina_requirement) {
            return Err("Failed to update stamina".into());
        }

        if !recipe.consumed_item_stacks.is_empty() {
            let consumed_item_stacks: Vec<ItemStack> = recipe
                .consumed_item_stacks
                .iter()
                .filter_map(|stack| {
                    let roll = ctx.rng().gen_range(0.0..=1.0);
                    if roll <= stack.consumption_chance {
                        Some(ItemStack::from(ctx, stack))
                    } else {
                        None
                    }
                })
                .collect();

            if let Err(err) =
                InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &consumed_item_stacks, |target| {
                    target.distance_to(placeable_coordinates)
                })
            {
                if consumed_item_stacks.is_empty() {
                    return Err(err);
                }

                return Err(format_missing_input_message(ctx, &consumed_item_stacks[0]));
            }
        }

        if recipe.tool_durability_lost > 0 {
            if let Some(tool_type) = used_tool_type {
                InventoryState::reduce_tool_durability(ctx, actor_id, tool_type, recipe.tool_durability_lost);
            }
        }

        let mut output = Vec::new();
        let mut damage_output = 0.0f32;
        let mut experience_damage_output = 0.0f32;
        let mut is_crit = false;

        if meets_level_requirements {
            let crit_multiplier = stats.get_final_crit_multiplier(ctx, recipe.get_skill_type());
            let base_damage = tool_power.round().max(1.0);
            let scaled_power = tool_power * recipe.power_multiplier;
            let damage = (scaled_power * crit_multiplier).round().max(1.0);
            let mut health = unwrap_or_err!(
                ctx.db.health_state().entity_id().find(&placeable.entity_id),
                "Placeable is missing health"
            );

            damage_output = health.health.min(damage);
            experience_damage_output = health.health.min(base_damage);
            is_crit = crit_multiplier > 1.0;

            let damage_multiplier = damage_output as i32;
            if damage_multiplier > 0 {
                for stack in &recipe.output_item_stacks {
                    let mut stack = stack.clone();
                    stack.quantity *= damage_multiplier;
                    if stack.quantity > 0 {
                        output.push(stack);
                    }
                }
            }

            for experience in &recipe.experience_per_progress {
                ExperienceState::add_experience_f32(ctx, actor_id, experience.skill_id, experience.quantity * experience_damage_output);
            }

            if damage_output > 0.0 {
                if let Some(self_buffs) = &recipe.self_buffs {
                    apply_self_buffs(ctx, actor_id, self_buffs)?;
                }
            }

            health.add_health_delta(-damage_output, ctx.timestamp);
            if health.health <= 0.0 {
                placeable.despawn(ctx);
                if let Some(outcome) = PlaceableState::pick_growth_outcome(ctx, recipe.on_destroy_outcomes) {
                    PlaceableState::spawn_growth_outcome_with_fallback(
                        ctx,
                        placeable.owner_entity_id,
                        placeable_coordinates,
                        placeable.direction_index,
                        &outcome,
                    )?;
                }
                PlayerActionState::success(
                    ctx,
                    actor_id,
                    PlayerActionType::None,
                    PlayerActionType::InteractPlaceable.get_layer(ctx),
                    0,
                    None,
                    None,
                    request.timestamp,
                );
            } else {
                ctx.db.health_state().entity_id().update(health);
            }
        }

        let mut extract_outcome: ExtractOutcomeStateV2 = ctx.db.extract_outcome_state().entity_id().find(&actor_id).unwrap();
        extract_outcome.target_entity_id = placeable.entity_id;
        extract_outcome.damage = damage_output as i32;
        extract_outcome.last_timestamp = ctx.timestamp;
        extract_outcome.is_crit = is_crit;
        ctx.db.extract_outcome_state().entity_id().update(extract_outcome);

        InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            actor_id,
            &output,
            |target| target.distance_to(placeable_coordinates),
            true,
            || vec![placeable_coordinates],
            false,
        )?;

        let _ = experience_damage_output;
    }

    Ok(())
}

fn validate_knowledges(
    ctx: &ReducerContext,
    actor_id: u64,
    required_knowledges: &[i32],
    blocking_knowledges: &[i32],
) -> Result<(), String> {
    for required_knowledge_id in required_knowledges {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *required_knowledge_id) {
            return Err("You don't have the knowledge required to perform this action".into());
        }
    }

    if !blocking_knowledges.is_empty() {
        let mut possess_all_knowledges = true;
        let secondary_knowledge = unwrap_or_err!(
            ctx.db.knowledge_secondary_state().entity_id().find(actor_id),
            "Player missing knowledge state"
        );
        for knowledge_id in blocking_knowledges {
            possess_all_knowledges &= secondary_knowledge
                .entries
                .iter()
                .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired);
        }
        if possess_all_knowledges {
            return Err("You don't need this placeable anymore".into());
        }
    }

    Ok(())
}
