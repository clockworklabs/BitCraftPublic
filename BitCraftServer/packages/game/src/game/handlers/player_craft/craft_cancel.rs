use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    building_desc, crafting_recipe_desc,
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    messages::{action_request::PlayerCraftCancelRequest, components::*, game_util::ItemStack},
    unwrap_or_err, CraftingRecipeDesc,
};

use super::craft_collect;

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_cancel(ctx: &ReducerContext, request: PlayerCraftCancelRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let action = unwrap_or_err!(
        ctx.db.progressive_action_state().entity_id().find(request.pocket_id),
        "Invalid pocket"
    );
    if action.recipe_id == 0 {
        return Err("Nothing to cancel".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(action.building_entity_id),
        "Invalid building"
    );

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Usage) {
        return Err("You don't have the permission to use this building".into());
    }

    if action.owner_entity_id != actor_id {
        return Err("You don't own this craft".into());
    }

    let player_coordinates = game_state_filters::coordinates_float(ctx, actor_id).into();

    // if player is not inside a building check if the request is for
    // an unenterable building (e.g. firepit) and if they are close enough
    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building.building_description_id),
        "Unknown building type"
    );
    if building_desc.unenterable {
        // Temporary: allow a distance of 2 for when you right-click on building while moving and end up 1 tile too far by completing the current move
        if building.distance_to(ctx, &player_coordinates) > 2 {
            return Err("Too far".into());
        }
    } else {
        let player_building = game_state_filters::building_at_coordinates(ctx, &player_coordinates);
        match player_building {
            Some(player_building) => {
                if player_building.entity_id != action.building_entity_id {
                    return Err("Player is inside another building".into());
                }
            }
            None => {
                return Err("Player isn't inside a building".into());
            }
        }
    }

    // Collect completed crafts
    let mut progressive_action = unwrap_or_err!(
        ctx.db.progressive_action_state().entity_id().find(request.pocket_id),
        "Invalid pocket"
    );
    let crafting_recipe_desc: CraftingRecipeDesc = unwrap_or_err!(
        ctx.db.crafting_recipe_desc().id().find(progressive_action.recipe_id),
        "Invalid recipe"
    );

    let completed_crafts = progressive_action.get_completed_crafts(crafting_recipe_desc.actions_required);
    let refunded_crafts = progressive_action.get_refunded_crafts(crafting_recipe_desc.actions_required);

    if completed_crafts > 0 {
        progressive_action.craft_count = completed_crafts;

        craft_collect::collect_output(ctx, actor_id, &progressive_action, &crafting_recipe_desc, &building)?;
    }

    if refunded_crafts > 0 {
        // Refund unspent materials
        // Note: This will repair any refunded durability item, but design promises content won't use durability items for craft inputs
        let refunds: Vec<ItemStack> = crafting_recipe_desc
            .consumed_item_stacks
            .iter()
            .map(|i| ItemStack::new(ctx, i.item_id, i.item_type, i.quantity * refunded_crafts))
            .collect();

        InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            actor_id,
            &refunds,
            |x| building.distance_to(ctx, &x),
            true,
            || vec![{ game_state_filters::coordinates_any(ctx, actor_id) }],
            false,
        )?;
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);
    ctx.db.progressive_action_state().entity_id().delete(request.pocket_id);
    ctx.db.public_progressive_action_state().entity_id().delete(request.pocket_id);

    Ok(())
}
