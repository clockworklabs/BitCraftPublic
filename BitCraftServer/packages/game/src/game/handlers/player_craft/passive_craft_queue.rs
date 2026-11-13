use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    building_desc, crafting_recipe_desc,
    game::{
        entities::building_state::InventoryState,
        game_state::{self, game_state_filters},
        handlers::player_craft::passive_craft_process::PassiveCraftTimer,
        reducer_helpers::{player_action_helpers, timer_helpers::now_plus_secs_f32},
    },
    messages::{
        action_request::PlayerPassiveCraftQueueRequest,
        components::*,
        game_util::{ItemStack, ItemType},
    },
    unwrap_or_err, BuildingFunction,
};

use super::passive_craft_process::passive_craft_timer;

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn passive_craft_queue(ctx: &ReducerContext, request: PlayerPassiveCraftQueueRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let recipe = unwrap_or_err!(ctx.db.crafting_recipe_desc().id().find(request.recipe_id), "Invalid recipe");

    let is_passive = recipe.is_passive && recipe.building_requirement.is_some();
    if !is_passive {
        return Err("This handler is for passive crafting only".into());
    }

    let building_state = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(request.building_entity_id),
        "Invalid building"
    );

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Usage) {
        return Err("You don't have the permission to use this building".into());
    }

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building_state.building_description_id),
        "Unknown building type"
    );

    if let Some(ref building_requirement) = recipe.building_requirement {
        if !building_desc.fulfills_function(building_requirement.building_type, building_requirement.tier) {
            return Err("Invalid building".into());
        }
    }

    // Temporary: allow a distance of 2 for when you right-click on building while moving and end up 1 tile too far by completing the current move
    if building_state.distance_to(ctx, &game_state_filters::coordinates_any(ctx, actor_id)) > 2 {
        return Err("Too far".into());
    }

    building_state.ensure_claim_tech(ctx)?;

    let building_crafts: Vec<PassiveCraftState> = ctx
        .db
        .passive_craft_state()
        .building_entity_id()
        .filter(request.building_entity_id)
        .collect();

    let num_player_crafts = building_crafts.iter().filter(|c| c.owner_entity_id == actor_id).count();
    let max_crafts = BuildingFunction::max_concurrent_crafts(&building_desc);
    if num_player_crafts as i32 >= max_crafts {
        return Err(format!("You can only have {{0}} passive crafts per building|~{}", max_crafts));
    }

    let item_stacks: Vec<ItemStack> = recipe.consumed_item_stacks.iter().map(|x| ItemStack::from(ctx, x)).collect();
    InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &item_stacks, |x| {
        building_state.distance_to(ctx, &x)
    })?;

    let entity_id = game_state::create_entity(ctx);
    let mut new_craft = PassiveCraftState {
        entity_id,
        owner_entity_id: actor_id,
        recipe_id: request.recipe_id,
        building_entity_id: request.building_entity_id,
        timestamp: ctx.timestamp,
        status: PassiveCraftStatus::Queued,
        slot: None,
    };

    // Try starting the craft right away
    let refining_slots_count = if recipe.crafted_item_stacks[0].item_type == ItemType::Item {
        building_desc
            .functions
            .iter()
            .max_by_key(|f| f.refining_slots)
            .unwrap()
            .refining_slots as u32
    } else {
        building_desc
            .functions
            .iter()
            .max_by_key(|f| f.refining_cargo_slots)
            .unwrap()
            .refining_cargo_slots as u32
    };
    if refining_slots_count == 0 {
        return Err("This building does not handle passive crafts".into());
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    let slots_in_use: Vec<u32> = building_crafts.iter().filter_map(|c| c.slot).collect();

    for i in 0..refining_slots_count {
        if !slots_in_use.contains(&i) {
            // Free slot, start the passive craft.
            // Note: a passive craft can only be queued if all the slots are full.
            // Once a passive craft finishes, the slot will start processing another queued one.
            // Therefore if a slot is empty at this point, the new craft goes in.
            new_craft.status = PassiveCraftStatus::Processing;
            new_craft.slot = Some(i);
            ctx.db
                .passive_craft_timer()
                .try_insert(PassiveCraftTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(
                        ctx.db
                            .crafting_recipe_desc()
                            .id()
                            .find(new_craft.recipe_id)
                            .unwrap()
                            .time_requirement,
                        ctx.timestamp,
                    ),
                    craft_entity_id: entity_id,
                })
                .ok()
                .unwrap();
            break;
        }
    }

    // Either processing or queued.
    let _ = ctx.db.passive_craft_state().try_insert(new_craft);

    Ok(())
}
