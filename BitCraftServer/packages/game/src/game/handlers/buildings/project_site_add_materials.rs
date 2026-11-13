use bitcraft_macro::feature_gate;
use crate::game::game_state::{self, game_state_filters};
use crate::game::handlers::inventory::inventory_helper;
use crate::game::permission_helper;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::PlayerProjectSiteAddMaterialsRequest;
use crate::messages::components::*;
use crate::messages::game_util::{InputItemStack, ItemStack, ItemType};
use crate::{construction_recipe_desc, resource_placement_recipe_desc, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn project_site_add_materials(ctx: &ReducerContext, request: PlayerProjectSiteAddMaterialsRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let location = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(request.owner_entity_id),
        "Invalid project site"
    );

    let coordinates = location.coordinates();

    if !PermissionState::can_interact_with_tile(ctx, actor_id, coordinates, Permission::Usage) {
        return Err("You don't have permission to interact with this project site".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, coordinates, actor_id, ClaimPermission::Usage) {
        return Err("You don't have permission to interact with this project site".into());
    }

    let mut project_site = unwrap_or_err!(
        ctx.db.project_site_state().entity_id().find(request.owner_entity_id),
        "Invalid project site"
    );

    if project_site.distance_to(ctx, game_state_filters::coordinates_any(ctx, actor_id)) > 2 {
        return Err("Too far".into());
    }

    let construction_recipe = ctx.db.construction_recipe_desc().id().find(project_site.construction_recipe_id);

    let resource_placement_recipe = ctx
        .db
        .resource_placement_recipe_desc()
        .id()
        .find(project_site.resource_placement_recipe_id);

    let consumed_cargo_stacks: Vec<InputItemStack>;
    let consumed_item_stacks: Vec<InputItemStack>;
    if let Some(recipe) = construction_recipe {
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        consumed_item_stacks = recipe.consumed_item_stacks;
    } else {
        let recipe = resource_placement_recipe.unwrap();
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        consumed_item_stacks = recipe.consumed_item_stacks;
    }

    let single_pocket_request = request.pockets.len() == 1;

    for from_pocket in &request.pockets {
        let mut inventory = unwrap_or_err!(
            ctx.db.inventory_state().entity_id().find(from_pocket.inventory_entity_id),
            "Missing inventory"
        );

        // Make sure player can currently interact with specified inventory
        inventory_helper::validate_interact(
            ctx,
            actor_id,
            location.coordinates(),
            inventory.owner_entity_id,
            inventory.player_owner_entity_id,
        )?;

        let pocket_index = from_pocket.pocket_index as usize;
        let mut item_stack = unwrap_or_err!(inventory.get_pocket_contents(pocket_index), "Missing material");

        if inventory.is_pocket_cargo(from_pocket.pocket_index as usize) {
            let cargo_id = item_stack.item_id;
            if cargo_id <= 0 {
                return Err("Invalid cargo".into());
            }

            let target = unwrap_or_err!(consumed_cargo_stacks.iter().find(|s| s.item_id == cargo_id), "Cargo not required");
            if target.quantity <= 0 {
                if single_pocket_request {
                    return Err("Cargo not required".into());
                }
                continue;
            }

            let available = inventory.get_at(pocket_index).unwrap().quantity;
            let cargos = &mut project_site.cargos;

            let current_position = cargos.iter().position(|s| s.item_id == cargo_id);
            if let Some(current_position) = current_position {
                let required = target.quantity - cargos[current_position].quantity;
                if required < 1 {
                    if single_pocket_request {
                        return Err("Cargo requirement already filled".into());
                    }
                    continue;
                }

                let quantity = available.min(required);
                cargos[current_position].quantity += quantity;
                inventory.remove_quantity_at(pocket_index, quantity);
            } else {
                let quantity = available.min(target.quantity);
                cargos.push(ItemStack::new(ctx, cargo_id, ItemType::Cargo, quantity));
                inventory.remove_quantity_at(pocket_index, quantity);
            };
        } else {
            let target = unwrap_or_err!(
                consumed_item_stacks.iter().find(|s| s.item_id == item_stack.item_id),
                "Item not required"
            );

            if target.quantity <= 0 {
                if single_pocket_request {
                    return Err("Item not required".into());
                }
                continue;
            }

            let items = &mut project_site.items;

            let current_position = items.iter().position(|s| s.item_id == item_stack.item_id);
            if let Some(current_position) = current_position {
                let quantity = i32::min(target.quantity - items[current_position].quantity, item_stack.quantity);
                if quantity <= 0 {
                    if single_pocket_request {
                        return Err("Item requirement already filled".into());
                    }
                    continue;
                }

                items[current_position].quantity += quantity;

                inventory.remove_quantity_at(pocket_index, quantity);
            } else {
                let quantity = i32::min(target.quantity, item_stack.quantity);
                item_stack.quantity = quantity;

                items.push(item_stack);

                inventory.remove_quantity_at(pocket_index, quantity);
            };
        }
        ctx.db.inventory_state().entity_id().update(inventory);
    }

    ctx.db.project_site_state().entity_id().update(project_site);

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
