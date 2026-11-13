use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::game::coordinates::*;
use crate::game::game_state::game_state_filters;
use crate::game::handlers::inventory::inventory_helper;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::PlayerRetrieveLostItemRequest;
use crate::messages::components::*;
use crate::messages::game_util::ItemType;
use crate::messages::static_data::{building_desc, BuildingCategory};
use crate::{game_state, params};
use crate::{parameters_desc, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn retrieve_lost_item(ctx: &ReducerContext, request: PlayerRetrieveLostItemRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(request.building_entity_id),
        "Invalid building"
    );

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building.building_description_id),
        "Unknown building type"
    );

    let player_coordinates = game_state_filters::coordinates_float(ctx, actor_id);
    if building.distance_to(ctx, &player_coordinates.into()) > 2 {
        return Err("Too far".into());
    }

    if !building_desc.has_category(ctx, BuildingCategory::RecoveryChest) {
        return Err("You can't retrieve items from this building".into());
    }

    if request.target_inventory_entity_id != 0 {
        let target_inventory = unwrap_or_err!(
            ctx.db.inventory_state().entity_id().find(request.target_inventory_entity_id),
            "Can't transfer items there"
        );

        if target_inventory.owner_entity_id != actor_id {
            let target_deployable = unwrap_or_err!(
                ctx.db.deployable_state_v2().entity_id().find(target_inventory.owner_entity_id),
                "Invalid transfer destination"
            );
            if target_deployable.owner_id != actor_id {
                return Err("That deployable is not yours".into());
            }

            let max_distance = ctx.db.parameters_desc().version().find(0).unwrap().withdraw_from_deployables_range;
            if game_state_filters::coordinates_float(ctx, target_deployable.entity_id).distance_to(player_coordinates) > max_distance as f32
            {
                return Err("Target inventory is too far".into());
            }
        }

        let target_inventory_type = inventory_helper::validate_interact(
            ctx,
            actor_id,
            player_coordinates.into(),
            target_inventory.owner_entity_id,
            target_inventory.player_owner_entity_id,
        )?;
        inventory_helper::validate_move(&target_inventory_type)?;
    }

    let building_coord: SmallHexTile = game_state_filters::coordinates_try_get(ctx, request.building_entity_id)?;
    let max_distance = params!(ctx).item_recovery_range;

    // TODO, possibly: sort by distance so the closest inventories get depleted first.
    for lost_items in ctx.db.lost_items_state().owner_entity_id().filter(actor_id) {
        let coord: SmallHexTile = lost_items.location.into();
        if building_coord.distance_to(coord) <= max_distance {
            let mut inventory = ctx.db.inventory_state().entity_id().find(lost_items.inventory_entity_id).unwrap();
            if inventory.entity_id == request.target_inventory_entity_id {
                continue;
            }

            let mut updated_inventory = false;
            for i in 0..inventory.pockets.len() {
                let pocket = inventory.pockets.get_mut(i).unwrap();
                if let Some(item_stack) = pocket.contents.as_mut() {
                    if item_stack.item_id == request.item_id && request.is_cargo == (item_stack.item_type == ItemType::Cargo) {
                        if let Some(durability) = item_stack.durability {
                            // durability items are only picked one by one if the durability matches the query
                                if durability == request.durability {
                                    if request.target_inventory_entity_id != 0 {
                                        let mut target_inventory = ctx
                                            .db
                                            .inventory_state()
                                            .entity_id()
                                            .find(request.target_inventory_entity_id)
                                            .unwrap();
                                        inventory_helper::validate_cargo_target(ctx, &target_inventory, item_stack)?;
                                        if !target_inventory.add(ctx, *item_stack) {
                                            return Err("Not enough room in inventory".into());
                                        }
                                    ctx.db.inventory_state().entity_id().update(target_inventory);
                                } else {
                                    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                                        ctx,
                                        actor_id,
                                        &vec![*item_stack],
                                        |x| building.distance_to(ctx, &x),
                                        false,
                                        || vec![{ game_state_filters::coordinates_any(ctx, actor_id) }],
                                        true,
                                    )?;
                                }
                                pocket.contents = None;
                                ctx.db.inventory_state().entity_id().update(inventory);
                                player_action_helpers::post_reducer_update_cargo(ctx, actor_id);
                                return Ok(());
                            }
                            continue;
                        }
                        // no durability items are always stacked
                        if request.target_inventory_entity_id != 0 {
                            let mut target_inventory = ctx
                                .db
                                .inventory_state()
                                .entity_id()
                                .find(request.target_inventory_entity_id)
                                .unwrap();
                            inventory_helper::validate_cargo_target(ctx, &target_inventory, item_stack)?;
                            if let Some(i) = request.target_inventory_index {
                                updated_inventory = target_inventory.add_partial_at(ctx, item_stack, i as usize);
                            } else {
                                updated_inventory = target_inventory.add_partial(ctx, item_stack);
                            }
                            ctx.db.inventory_state().entity_id().update(target_inventory);
                        } else {
                            let overflow_items = InventoryState::deposit_to_player_inventory_and_nearby_deployables_and_get_overflow_stack(
                                ctx,
                                actor_id,
                                &vec![*item_stack],
                                |x| building.distance_to(ctx, &x),
                                true,
                            )?;
                            if !overflow_items.is_empty() {
                                item_stack.quantity = overflow_items[0].quantity;
                            } else {
                                pocket.contents = None;
                            }
                            updated_inventory = true;
                        }
                    }
                }
            }
            if updated_inventory {
                if inventory.is_empty() {
                    ctx.db.inventory_state().entity_id().delete(inventory.entity_id);
                    ctx.db.lost_items_state().inventory_entity_id().delete(inventory.entity_id);
                } else {
                    ctx.db.inventory_state().entity_id().update(inventory);
                }
            }
        }
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
