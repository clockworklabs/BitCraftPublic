use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    building_desc, crafting_recipe_desc,
    game::{
        entities::building_state::BuildingState,
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    messages::{
        action_request::{PlayerCraftCollectAllRequest, PlayerCraftCollectRequest},
        components::*,
        game_util::ItemStack,
        static_data::CraftingRecipeDesc,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_collect(ctx: &ReducerContext, request: PlayerCraftCollectRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let progressive_action = unwrap_or_err!(
        ctx.db.progressive_action_state().entity_id().find(request.pocket_id),
        "Invalid pocket"
    );

    if progressive_action.recipe_id == 0 {
        return Err("Nothing to collect".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(progressive_action.building_entity_id),
        "Invalid building"
    );
    if building.distance_to(ctx, &game_state_filters::coordinates_float(ctx, actor_id).into()) > 2 {
        return Err("Too far".into());
    }

    if progressive_action.owner_entity_id != actor_id {
        return Err("You don't own this craft".into());
    }

    if request.recipe_id != progressive_action.recipe_id {
        return Err("Recipe mismatch".into());
    }

    let crafting_recipe_desc: CraftingRecipeDesc = unwrap_or_err!(
        ctx.db.crafting_recipe_desc().id().find(progressive_action.recipe_id),
        "Invalid recipe"
    );

    if progressive_action.get_status_from_recipe(&crafting_recipe_desc, ctx.timestamp) != ProgressiveActionStatus::Completed {
        return Err("Recipe not fully crafted yet".into());
    }

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Usage) {
        return Err("You don't have the permission to collect from this building".into());
    }

    can_interact_with_building(ctx, &actor_id, &progressive_action.building_entity_id)?;

    collect_output(ctx, actor_id, &progressive_action, &crafting_recipe_desc, &building)?;
    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_collect_all(ctx: &ReducerContext, request: PlayerCraftCollectAllRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    can_interact_with_building(ctx, &actor_id, &request.building_entity_id)?;

    let mut collected_any = false;

    for progressive_action_state in ctx
        .db
        .progressive_action_state()
        .building_entity_id()
        .filter(request.building_entity_id)
    {
        let crafting_recipe_desc = match ctx.db.crafting_recipe_desc().id().find(progressive_action_state.recipe_id) {
            Some(value) => value,
            None => continue,
        };

        let building = unwrap_or_err!(
            ctx.db
                .building_state()
                .entity_id()
                .find(progressive_action_state.building_entity_id),
            "Invalid building"
        );
        if building.distance_to(ctx, &game_state_filters::coordinates_float(ctx, actor_id).into()) > 2 {
            return Err("Too far".into());
        }

        if progressive_action_state.owner_entity_id != actor_id {
            continue;
        }

        if progressive_action_state.get_status_from_recipe(&crafting_recipe_desc, ctx.timestamp) != ProgressiveActionStatus::Completed {
            continue;
        }

        collect_output(ctx, actor_id, &progressive_action_state, &crafting_recipe_desc, &building)?;

        collected_any = true;
    }

    if !collected_any {
        return Err("Nothing to collect".into());
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

fn can_interact_with_building(ctx: &ReducerContext, actor_id: &u64, building_entity_id: &u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, *actor_id, true)?;

    let building = unwrap_or_err!(ctx.db.building_state().entity_id().find(building_entity_id), "Invalid building");

    let player_coordinates = game_state_filters::coordinates_float(ctx, *actor_id).into();

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
                if player_building.entity_id != *building_entity_id {
                    return Err("Player is inside another building".into());
                }
            }
            None => {
                return Err("Player isn't inside a building".into());
            }
        }
    }

    Ok(())
}

pub fn collect_output(
    ctx: &ReducerContext,
    actor_id: u64,
    progressive_action_state: &ProgressiveActionState,
    crafting_recipe_desc: &CraftingRecipeDesc,
    building_state: &BuildingState,
) -> Result<(), String> {
    let output = crafting_recipe_desc
        .crafted_item_stacks
        .iter()
        .map(|is| ItemStack::new(ctx, is.item_id, is.item_type, is.quantity * progressive_action_state.craft_count))
        .collect();

    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        actor_id,
        &output,
        |x| building_state.distance_to(ctx, &x),
        true,
        || vec![{ game_state_filters::coordinates_any(ctx, actor_id) }],
        false,
    )?;

    let action_entity_id = progressive_action_state.entity_id;
    ctx.db.progressive_action_state().entity_id().delete(action_entity_id);
    ctx.db.public_progressive_action_state().entity_id().delete(action_entity_id);

    Ok(())
}
