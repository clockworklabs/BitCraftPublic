use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    building_desc, crafting_recipe_desc,
    game::game_state::{self, game_state_filters},
    messages::{components::*, game_util::ItemStack},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn passive_craft_cancel(ctx: &ReducerContext, passive_craft_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let passive_craft = unwrap_or_err!(
        ctx.db.passive_craft_state().entity_id().find(passive_craft_entity_id),
        "Project no longer exists"
    );

    let slot = passive_craft.slot;

    if passive_craft.owner_entity_id != actor_id {
        return Err("This is not your yours to cancel.".into());
    }

    let building_entity_id = passive_craft.building_entity_id;
    let building_state = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(building_entity_id),
        "Building does not exist"
    );

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Usage) {
        return Err("You don't have the permission to use this building".into());
    }

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building_state.building_description_id),
        "Unknown building type"
    );

    if building_desc.unenterable {
        // Temporary: allow a distance of 2 for when you right-click on building while moving and end up 1 tile too far by completing the current move
        if building_state.distance_to(ctx, &game_state_filters::coordinates_any(ctx, actor_id)) > 2 {
            return Err("Too far".into());
        }
    } else {
        return Err("Player isn't inside a building".into());
    }

    //Refund materials
    if let Some(crafting_recipe) = ctx.db.crafting_recipe_desc().id().find(passive_craft.recipe_id) {
        if let Some(mut player_inventory) = InventoryState::get_player_inventory(ctx, actor_id) {
            let item_stacks = crafting_recipe
                .consumed_item_stacks
                .iter()
                .map(|x| ItemStack::from(ctx, x))
                .collect();

            player_inventory.add_multiple_with_overflow(ctx, &item_stacks);
            ctx.db.inventory_state().entity_id().update(player_inventory);
        }
    }

    ctx.db.passive_craft_state().entity_id().delete(passive_craft_entity_id);

    // Try starting the craft right away in the same slot (other slots will have filled automatically)
    if let Some(slot) = slot {
        PassiveCraftState::process_oldest_queued(ctx, building_entity_id, slot, ctx.timestamp);
    }

    Ok(())
}
