use spacetimedb::ReducerContext;

use crate::{
    game::{
        entities::{buff, building_state::InventoryState},
        reducer_helpers::player_action_helpers::post_reducer_update_cargo,
    },
    messages::{components::*, static_data::FoodDesc},
    unwrap_or_err, CharacterStatType,
};

pub fn consume(ctx: &ReducerContext, actor_id: u64, food: &FoodDesc, quantity: i32) -> Result<(), String> {
    if quantity <= 0 {
        return Ok(());
    }

    let character_stats_state = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player has no stats");

    for _ in 0..quantity {
        if food.hp != 0f32 {
            apply_health(ctx, actor_id, food, &character_stats_state)?;
        }

        if food.stamina != 0f32 {
            apply_stamina(ctx, actor_id, food, &character_stats_state)?;
        }

        if food.hunger != 0f32 {
            SatiationState::add_player_satiation(ctx, actor_id, food.hunger);
        }

        if food.teleportation_energy != 0f32 {
            let mut teleportation_energy_state = TeleportationEnergyState::get(ctx, actor_id);
            if teleportation_energy_state.add_energy(ctx, food.teleportation_energy) {
                teleportation_energy_state.update(ctx);
            }
        }

        for buff_effect in &food.buffs {
            buff::activate(ctx, actor_id, buff_effect.buff_id, buff_effect.duration, None)?;
        }
    }

    if let Some(output_item_stacks) = &food.output_item_stacks {
        let mut output = Vec::new();
        for _ in 0..quantity {
            output.extend(output_item_stacks.iter().cloned());
        }

        if !output.is_empty() {
            let player_location = unwrap_or_err!(
                ctx.db.mobile_entity_state().entity_id().find(&actor_id),
                "Unknown player location"
            )
            .coordinates();

            InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                ctx,
                actor_id,
                &output,
                |coordinates| coordinates.distance_to(player_location),
                true,
                || vec![player_location],
                false,
            )?;
            post_reducer_update_cargo(ctx, actor_id);
        }
    }

    Ok(())
}

fn apply_health(ctx: &ReducerContext, actor_id: u64, food: &FoodDesc, character_stats_state: &CharacterStatsState) -> Result<(), String> {
    let mut health_state = unwrap_or_err!(ctx.db.health_state().entity_id().find(&actor_id), "Player has no health state");

    if food.up_to_hp == 0f32 {
        if health_state.add_health_delta_clamped(
            food.hp,
            0f32,
            character_stats_state.get(CharacterStatType::MaxHealth),
            ctx.timestamp,
        ) {
            ctx.db.health_state().entity_id().update(health_state);
        }
    } else if food.hp > 0f32 {
        if health_state.add_health_delta_clamped(food.hp, 0f32, food.up_to_hp, ctx.timestamp) {
            ctx.db.health_state().entity_id().update(health_state);
        }
    } else if health_state.add_health_delta_clamped(
        food.hp,
        food.up_to_hp,
        character_stats_state.get(CharacterStatType::MaxHealth),
        ctx.timestamp,
    ) {
        ctx.db.health_state().entity_id().update(health_state);
    }

    Ok(())
}

fn apply_stamina(ctx: &ReducerContext, actor_id: u64, food: &FoodDesc, character_stats_state: &CharacterStatsState) -> Result<(), String> {
    let mut stamina_state = unwrap_or_err!(ctx.db.stamina_state().entity_id().find(&actor_id), "Player has no stamina state");

    if food.up_to_stamina == 0f32 {
        if stamina_state.add_stamina_delta_clamped(
            food.stamina,
            0f32,
            character_stats_state.get(CharacterStatType::MaxStamina),
            ctx.timestamp,
        ) {
            ctx.db.stamina_state().entity_id().update(stamina_state);
        }
    } else if food.stamina > 0f32 {
        if stamina_state.add_stamina_delta_clamped(food.stamina, 0f32, food.up_to_stamina, ctx.timestamp) {
            ctx.db.stamina_state().entity_id().update(stamina_state);
        }
    } else if stamina_state.add_stamina_delta_clamped(
        food.stamina,
        food.up_to_stamina,
        character_stats_state.get(CharacterStatType::MaxStamina),
        ctx.timestamp,
    ) {
        ctx.db.stamina_state().entity_id().update(stamina_state);
    }

    Ok(())
}
