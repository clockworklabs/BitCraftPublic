use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{rand::Rng, ReducerContext};

use crate::game::game_state::{self, game_state_filters};
use crate::game::handlers::{
    player::{extract, placeable_interact},
    player_craft::craft,
};
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::components::{
    active_buff_state, enemy_state, health_state, player_action_state, player_state, stamina_state, AbilityState, AbilityType,
    AbilityTypeEnum, PlaceableState, PlayerActionResult, PlayerActionState, PlayerActionType, PlayerTimestampState, ThreatState,
};
use crate::messages::game_util::ItemStack;
use crate::messages::static_data::{
    ability_custom_desc, crafting_recipe_desc, extraction_recipe_desc, placeable_desc, placeable_interaction_desc, AbilityCustomDesc,
    AbilityUnlockDesc, SkillType,
};
use crate::{unwrap_or_err, InventoryState};

#[spacetimedb::reducer]
#[feature_gate]
pub fn ability_custom_activate_start(
    ctx: &ReducerContext,
    ability_custom_id: i32,
    target_entity_id: u64,
    timestamp: u64,
) -> Result<(), String> {
    let custom_effect = unwrap_or_err!(ctx.db.ability_custom_desc().id().find(ability_custom_id), "Unknown Custom Effect");

    if custom_effect.cast_time == 0.0 {
        return ability_custom_activate(ctx, ability_custom_id, target_entity_id, timestamp);
    }

    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let delay = Duration::from_secs_f32(custom_effect.cast_time);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::AbilityCustom,
        Some(target_entity_id),
        None,
        delay,
        reduce(ctx, &custom_effect, target_entity_id, timestamp, true),
        timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn ability_custom_activate(ctx: &ReducerContext, ability_custom_id: i32, target_entity_id: u64, timestamp: u64) -> Result<(), String> {
    let custom_effect = unwrap_or_err!(ctx.db.ability_custom_desc().id().find(ability_custom_id), "Unknown Custom Effect");
    reduce(ctx, &custom_effect, target_entity_id, timestamp, false)
}

pub fn reduce(
    ctx: &ReducerContext,
    custom_effect: &AbilityCustomDesc,
    target_entity_id: u64,
    timestamp: u64,
    dry_run: bool,
) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    let ability_data = AbilityType::Custom(custom_effect.id);

    // Validate if ability is locked
    AbilityUnlockDesc::evaluate(ctx, actor_id, AbilityTypeEnum::Custom, ability_data)?;

    // Any one active buff from the authored list satisfies the requirement. List order also
    // determines which buff is consumed when multiple acceptable buffs are active.
    let required_buff_id = match &custom_effect.required_buff_ids {
        Some(required_buff_ids) if !required_buff_ids.is_empty() => {
            let active_buff_state = unwrap_or_err!(
                ctx.db.active_buff_state().entity_id().find(actor_id),
                "Player has no ActiveBuffState."
            );
            match required_buff_ids
                .iter()
                .copied()
                .find(|buff_id| active_buff_state.has_active_buff(*buff_id, ctx.timestamp))
            {
                Some(buff_id) => Some(buff_id),
                None => return Err("Required buff is not active".into()),
            }
        }
        _ => None,
    };

    let profession_action = if custom_effect.perform_profession_hit {
        if custom_effect.cast_time != 0.0 {
            return Err("Profession hit abilities must be instant".into());
        }

        let mut profession_actions = ctx.db.player_action_state().entity_id().filter(actor_id).filter(|action| {
            (action.action_type == PlayerActionType::Craft
                || action.action_type == PlayerActionType::Extract
                || action.action_type == PlayerActionType::InteractPlaceable)
                && action.last_action_result == PlayerActionResult::Success
                && !action.was_consumed
        });
        let profession_action = profession_actions
            .next()
            .ok_or_else(|| "Not currently crafting or gathering".to_string())?;
        if profession_actions.next().is_some() {
            return Err("Multiple profession actions are active".into());
        }

        let recipe_skill_id = match profession_action.action_type {
            PlayerActionType::Craft => ctx
                .db
                .crafting_recipe_desc()
                .id()
                .find(
                    profession_action
                        .recipe_id
                        .ok_or_else(|| "Current crafting action has no recipe".to_string())?,
                )
                .ok_or_else(|| "Current crafting recipe was not found".to_string())?
                .level_requirements
                .first()
                .map(|requirement| requirement.skill_id)
                .unwrap_or(SkillType::None as i32),
            PlayerActionType::Extract => ctx
                .db
                .extraction_recipe_desc()
                .id()
                .find(
                    profession_action
                        .recipe_id
                        .ok_or_else(|| "Current gathering action has no recipe".to_string())?,
                )
                .ok_or_else(|| "Current gathering recipe was not found".to_string())?
                .level_requirements
                .first()
                .map(|requirement| requirement.skill_id)
                .unwrap_or(SkillType::None as i32),
            PlayerActionType::InteractPlaceable => ctx
                .db
                .placeable_interaction_desc()
                .id()
                .find(
                    profession_action
                        .recipe_id
                        .ok_or_else(|| "Current placeable interaction has no recipe".to_string())?,
                )
                .ok_or_else(|| "Current placeable interaction recipe was not found".to_string())?
                .level_requirements
                .first()
                .map(|requirement| requirement.skill_id)
                .unwrap_or(SkillType::None as i32),
            _ => return Err("Not currently crafting or gathering".into()),
        };
        if custom_effect.profession_hit_skill_id != SkillType::ANY as i32 && custom_effect.profession_hit_skill_id != recipe_skill_id {
            return Err("Profession hit ability does not match the current recipe skill".into());
        }
        Some(profession_action)
    } else {
        None
    };

    // Validate timestamps
    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::AbilityCustom, timestamp)?;
    if !dry_run && custom_effect.cast_time > 0.0 {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::AbilityCustom, Some(target_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::AbilityCustom, timestamp)?;
    }

    // Check ability cooldown (only in dry-run, check is done as we set the cooldown in the real run)
    let ability_state = AbilityState::get(ctx, actor_id, ability_data);

    if dry_run && ability_state.is_under_cooldown(ctx, true) {
        return Err("Ability is under cooldown".into());
    }

    // Expend health
    if custom_effect.health_cost > 0 {
        let mut health = ctx.db.health_state().entity_id().find(actor_id).unwrap();
        if (health.health as i32) <= custom_effect.health_cost {
            return Err("Not enough health to trigger this ability".into());
        }
        if !dry_run {
            health.add_health_delta(-custom_effect.health_cost as f32, ctx.timestamp);
            ctx.db.health_state().entity_id().update(health);
        }
    }

    // Expend stamina
    if custom_effect.stamina_cost > 0 {
        let mut stamina = ctx.db.stamina_state().entity_id().find(actor_id).unwrap();
        if (stamina.stamina as i32) < custom_effect.stamina_cost {
            return Err("Not enough stamina to trigger this ability".into());
        }
        if !dry_run {
            stamina.stamina -= custom_effect.stamina_cost as f32;
            ctx.db.stamina_state().entity_id().update(stamina);
        }
    }

    // Validate distance
    let actor_location = game_state_filters::coordinates_float(ctx, actor_id);
    let target_location = game_state_filters::coordinates_float(ctx, target_entity_id);
    if actor_location.distance_to(target_location) > custom_effect.range + 3.0 {
        return Err("Too far".into());
    }

    // Validate target type
    // TODO - Enemies triggering abilities?
    if custom_effect.friendly {
        if ctx.db.player_state().entity_id().find(target_entity_id).is_none() {
            return Err("Invalid target".into());
        }
    } else {
        if ctx.db.enemy_state().entity_id().find(target_entity_id).is_none() {
            return Err("Invalid target".into());
        }
    }

    let actor_coordinates = actor_location.parent_small_tile();
    for spawned_placeable in custom_effect.spawned_placeables.as_deref().unwrap_or_default() {
        if spawned_placeable.placeable_id == 0
            || !(0.0..=1.0).contains(&spawned_placeable.chance)
            || spawned_placeable.radius_min < 0
            || spawned_placeable.radius_max < spawned_placeable.radius_min
        {
            return Err("Custom ability has an invalid spawned placeable".into());
        }
        if ctx.db.placeable_desc().id().find(spawned_placeable.placeable_id).is_none() {
            return Err("Custom ability references an unknown spawned placeable".into());
        }
    }
    let input_item_stacks = custom_effect.input_item_stacks.as_deref().unwrap_or_default();
    for input in input_item_stacks {
        if input.quantity <= 0 || !(0.0..=1.0).contains(&input.consumption_chance) {
            return Err("Custom ability has an invalid input stack".into());
        }
    }
    let required_input_stacks = ItemStack::merge_multiple(&input_item_stacks.iter().map(|input| ItemStack::from(ctx, input)).collect());
    if !InventoryState::has_items_in_player_inventory_and_nearby_deployables(ctx, actor_id, &required_input_stacks, |coordinates| {
        coordinates.distance_to(actor_coordinates)
    })? {
        return Err("You don't have the required items".into());
    }

    if dry_run {
        return Ok(());
    }

    let consumed_input_stacks = ItemStack::merge_multiple(
        &input_item_stacks
            .iter()
            .filter_map(|input| (ctx.rng().gen_range(0.0..1.0) < input.consumption_chance).then(|| ItemStack::from(ctx, input)))
            .collect(),
    );
    InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &consumed_input_stacks, |coordinates| {
        coordinates.distance_to(actor_coordinates)
    })?;

    if custom_effect.consume_required_buff {
        if let Some(required_buff_id) = required_buff_id {
            let mut active_buff_state = unwrap_or_err!(
                ctx.db.active_buff_state().entity_id().find(actor_id),
                "Player has no ActiveBuffState."
            );
            active_buff_state.remove_active_buff(ctx, required_buff_id);
            ctx.db.active_buff_state().entity_id().update(active_buff_state);
        }
    }

    // Apply cooldowns
    if !ability_state.set_cooldown(ctx, custom_effect.cooldown, custom_effect.global_cooldown, true) {
        return Err("Ability is under cooldown".into());
    }

    if let Some(profession_action) = profession_action {
        match profession_action.action_type {
            PlayerActionType::Craft => craft::apply_ability_profession_hit(
                ctx,
                actor_id,
                &profession_action,
                custom_effect.profession_hit_bonus_damage,
                timestamp,
            )?,
            PlayerActionType::Extract => extract::apply_ability_profession_hit(
                ctx,
                actor_id,
                &profession_action,
                custom_effect.profession_hit_bonus_damage,
                timestamp,
            )?,
            PlayerActionType::InteractPlaceable => placeable_interact::apply_ability_profession_hit(
                ctx,
                actor_id,
                &profession_action,
                custom_effect.profession_hit_bonus_damage,
                timestamp,
            )?,
            _ => return Err("Not currently crafting or gathering".into()),
        }
    }

    // TODO - Radius + friendly
    let targets = vec![target_entity_id];

    for target in targets {
        // Apply buffs
        if custom_effect.buffs.len() > 0 {
            let mut active_buff_state = unwrap_or_err!(
                ctx.db.active_buff_state().entity_id().find(target),
                "Entity has no ActiveBuffState."
            );
            for buff_effect in &custom_effect.buffs {
                if custom_effect.buff_toggle && active_buff_state.has_active_buff(buff_effect.buff_id, ctx.timestamp) {
                    // toggle of existing buff
                    active_buff_state.remove_active_buff(ctx, buff_effect.buff_id);
                } else {
                    // apply buff
                    active_buff_state.add_active_buff_with_data(ctx, buff_effect.buff_id, buff_effect.duration, None);
                }
            }
            ctx.db.active_buff_state().entity_id().update(active_buff_state);
        }

        // Apply damage / healing
        if custom_effect.damage != 0 {
            let mut health = ctx.db.health_state().entity_id().find(target).unwrap();
            health.add_health_delta(-custom_effect.health_cost as f32, ctx.timestamp);
            ctx.db.health_state().entity_id().update(health);
        }

        // Apply threat
        if custom_effect.threat_value != 0.0 {
            ThreatState::add_threat(ctx, actor_id, target, custom_effect.threat_value);
        }
    }

    let mut output_item_stacks = Vec::new();
    for output in custom_effect.output_item_stacks.as_deref().unwrap_or_default() {
        if !(0.0..=1.0).contains(&output.probability) {
            return Err("Custom ability has an invalid output chance".into());
        }
        let item_stack = output
            .item_stack
            .ok_or_else(|| "Custom ability has an invalid output stack".to_string())?;
        if item_stack.quantity <= 0 {
            return Err("Custom ability has an invalid output quantity".into());
        }
        if ctx.rng().gen_range(0.0..1.0) < output.probability {
            output_item_stacks.push(item_stack);
        }
    }
    let output_item_stacks = ItemStack::merge_multiple(&output_item_stacks);
    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        actor_id,
        &output_item_stacks,
        |coordinates| coordinates.distance_to(actor_coordinates),
        true,
        || vec![actor_coordinates],
        false,
    )?;
    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    for spawned_placeable in custom_effect.spawned_placeables.as_deref().unwrap_or_default() {
        if spawned_placeable.chance > 0.0 && (spawned_placeable.chance >= 1.0 || ctx.rng().gen_range(0.0..1.0) < spawned_placeable.chance) {
            let _ = PlaceableState::spawn_in_radius_band(
                ctx,
                spawned_placeable.placeable_id,
                actor_id,
                actor_coordinates,
                0,
                spawned_placeable.radius_min,
                spawned_placeable.radius_max,
            );
        }
    }

    // Apply Linked effect
    if custom_effect.linked_ability_buff_desc_id != 0 {
        let custom_effect = unwrap_or_err!(
            ctx.db.ability_custom_desc().id().find(custom_effect.linked_ability_buff_desc_id),
            "Unknown Linked Custom Effect"
        );
        return reduce(ctx, &custom_effect, target_entity_id, timestamp, dry_run);
    }

    Ok(())
}
