use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::entities::inventory_type;
use crate::game::game_state;
use crate::game::handlers::inventory::inventory_helper;
use crate::game::reducer_helpers::{loot_chest_helpers, player_action_helpers};
use crate::messages::action_request::PlayerPocketSwapContentsRequest;
use crate::messages::components::*;
use crate::messages::game_util::*;
use crate::messages::static_data::ToolTypeDesc;
use crate::{
    building_desc, cargo_desc, equipment_desc, item_desc, skill_desc, tool_desc, unwrap_or_err, weapon_desc, weapon_type_desc,
    AchievementDesc, BuildingFunction,
};
use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn pocket_swap_contents(ctx: &ReducerContext, request: PlayerPocketSwapContentsRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, &request.from_pocket, &request.to_pocket, request.quantity)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, from_pocket: &PocketKey, to_pocket: &PocketKey, quantity: i32) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if quantity <= 0 {
        return Err("Invalid quantity".into());
    }

    let same_inventory = from_pocket.inventory_entity_id == to_pocket.inventory_entity_id;

    let mut from_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(from_pocket.inventory_entity_id),
        "Invalid source inventory"
    );

    let mut to_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(to_pocket.inventory_entity_id),
        "Invalid target inventory"
    );

    let to_inventory_owner_entity_id = to_inventory.owner_entity_id;
    let from_inventory_owner_entity_id = from_inventory.owner_entity_id;

    let player_location = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Player has no location").coordinates();

    let from_inventory_type = inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        from_inventory.owner_entity_id,
        from_inventory.player_owner_entity_id,
    )?;

    let to_inventory_type = inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        to_inventory.owner_entity_id,
        to_inventory.player_owner_entity_id,
    )?;

    inventory_helper::validate_swap(&from_inventory_type, &to_inventory_type)?;

    let to_index = to_pocket.pocket_index as usize;
    let to_empty = to_inventory.is_pocket_empty(to_index);

    let is_cargo = from_inventory.is_pocket_cargo(from_pocket.pocket_index as usize);
    if is_cargo != to_inventory.is_pocket_cargo(to_pocket.pocket_index as usize) {
        return Err("Can't swap between cargo and item".into());
    }

    let from_index = from_pocket.pocket_index as usize;
    let from_contents = unwrap_or_err!(from_inventory.get_pocket_contents(from_index), "Invalid origin");
    if from_contents.item_id <= 0 || from_contents.quantity <= 0 {
        return Err("Invalid source pocket.".into());
    }
    if from_contents.quantity < quantity {
        return Err("Invalid quantity".into());
    }

    if from_pocket.inventory_entity_id == to_pocket.inventory_entity_id && from_pocket.pocket_index == to_pocket.pocket_index {
        spacetimedb::log::warn!("Player {} possibly tried to dupe items using pocket_swap_contents (inventory: {}, pocket: {}, quantity: {}, pocket contents: {}x{})", actor_id, from_pocket.inventory_entity_id, from_pocket.pocket_index, quantity, from_contents.quantity, from_contents.item_id);
        return Ok(());
    }

    let tool_info = ctx.db.tool_desc().item_id().filter(from_contents.item_id).next();
    let weapon_info = ctx.db.weapon_desc().item_id().find(from_contents.item_id);
    let is_tool = tool_info.is_some();
    let is_weapon = weapon_info.is_some();
    let to_toolbelt = to_inventory.inventory_index == 1 && to_inventory.owner_entity_id == actor_id;
    let from_toolbelt = from_inventory.inventory_index == 1 && from_inventory.owner_entity_id == actor_id;

    let is_allowed_on_toolbelt = is_tool || is_weapon;
    if to_toolbelt {
        if !is_allowed_on_toolbelt {
            return Err("Only tools or weapons are allowed on the toolbelt".into());
        }
        // Weapon always goes on last pocket
        if to_pocket.pocket_index == ToolTypeDesc::get_combat_weapon_slot(ctx) {
            if let Some(info) = weapon_info {
                let weapon_type = ctx.db.weapon_type_desc().id().find(info.weapon_type).unwrap();
                if weapon_type.hunting {
                    return Err("Hunting tools can't be equipped there.".into());
                }
            } else {
                return Err("Only weapons go on this pocket.".into());
            }
        } else {
            if to_pocket.pocket_index != ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool_info.unwrap().tool_type) {
                return Err("This tool doesn't go on this pocket.".into());
            }
        }
    }

    let item_id = from_contents.item_id;

    let to_pocket = to_inventory.pockets[to_index].clone();

    // only allow the right item ids in wallet
    if to_inventory.inventory_index == 2 {
        if let Some(wallet_pocket) = to_inventory.wallet_pocket_target(ctx, &from_contents) {
            if wallet_pocket != to_index {
                return Err("Not allowed in that wallet slot".into());
            }
        } else {
            return Err("Not allowed in the wallet.".into());
        }
    }

    // only allow swapping back the right item id in the right wallet pocket
    if from_inventory.inventory_index == 2 && !to_empty {
        if let Some(wallet_pocket) = from_inventory.wallet_pocket_target(ctx, &to_pocket.contents.unwrap()) {
            if wallet_pocket != from_pocket.pocket_index as usize {
                return Err("Not allowed in that wallet slot".into());
            }
        } else {
            return Err("Not allowed in the wallet.".into());
        }
    }

    let target_pocket_quantity = match to_pocket.contents {
        Some(c) => c.quantity,
        None => 0,
    };

    // if the inventory is a stock-pile with locked cargo indexes, we need to make sure the destination is the right type of cargo
    let volume;

    let mut discovery = Discovery::new(actor_id);

    if is_cargo {
        inventory_helper::validate_cargo_target(ctx, &to_inventory, &from_contents)?;

        if let Some(building) = ctx.db.building_state().entity_id().find(to_inventory.owner_entity_id) {
            let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
            if let Some(function) = BuildingFunction::from_inventory_index(&building_desc, to_inventory.inventory_index) {
                // Typed storage, each slot can only receive a matching cargo id
                if let Some(required_cargo_id) = function.allowed_item_id_per_slot.get(to_index) {
                    if from_contents.item_id != *required_cargo_id {
                        let cargo_name = ctx.db.cargo_desc().id().find(required_cargo_id).unwrap().name;
                        let err_msg = format!("You can only store {{0}} in that slot.|~{}", cargo_name);
                        return Err(err_msg);
                    }
                }
            }
        }
        volume = match ctx.db.cargo_desc().id().find(item_id) {
            Some(c) => c.volume,
            None => return Err("Invalid source pocket".into()), // invalid cargo id, probably 0.
        };
    } else {
        if let Some(item_desc) = ctx.db.item_desc().id().find(item_id) {
            volume = item_desc.volume;
        } else {
            return Err("Invalid source pocket".into()); // invalid item id, probably 0.
        }
    }

    let inserted_quantity = std::cmp::min(to_pocket.can_fit_quantity(ctx, volume, is_cargo), quantity);

    let mut quantity = target_pocket_quantity + inserted_quantity;
    if let Some(pocket_contents) = to_pocket.contents {
        if pocket_contents.item_id != from_contents.item_id {
            quantity = from_contents.quantity;
        }
    }

    let item_stack = from_contents.clone_with_quantity(quantity);

    if to_empty {
        //remove
        //Note: no need to set_at_and_discover here. Add handles this.
        from_inventory.set_at(
            from_index,
            Some(item_stack.clone_with_quantity(from_contents.quantity - inserted_quantity)),
        );

        //add
        if same_inventory {
            //Note: use set_at_and_discover to support item list movement
            InventoryState::set_at_and_discover(
                ctx,
                to_index,
                Some(item_stack),
                &mut discovery,
                &mut from_inventory,
                from_inventory_owner_entity_id == actor_id,
            );
        } else {
            //Note: use set_at_and_discover to support item list movement
            InventoryState::set_at_and_discover(
                ctx,
                to_index,
                Some(item_stack),
                &mut discovery,
                &mut to_inventory,
                to_inventory_owner_entity_id == actor_id,
            );
        }
    } else {
        let to_contents = to_inventory.get_pocket_contents(to_index).unwrap();
        if is_cargo {
            inventory_helper::validate_cargo_target(ctx, &from_inventory, &to_contents)?;
        }

        if from_contents.item_id != to_contents.item_id {
            if is_cargo && to_contents.quantity > 1
                && (from_inventory.owner_entity_id == actor_id || to_inventory.owner_entity_id == actor_id) {
                    return Err("Cannot swap a stack containing multiple cargos to your inventory".into());
                }

            if from_contents.quantity != quantity {
                return Err("Invalid target pocket".into());
            }

            // Make sure the swapped item doesn't end up illegally on the toolbelt
            let tool_info = ctx.db.tool_desc().item_id().filter(to_contents.item_id);
            let weapon_info = ctx.db.weapon_desc().item_id().find(to_contents.item_id);
            let is_tool = tool_info.count() > 0;
            let is_weapon = weapon_info.is_some();
            let is_allowed_on_toolbelt = is_tool || is_weapon;
            if from_toolbelt && !is_allowed_on_toolbelt {
                return Err("Only tools or weapons are allowed on the toolbelt".into());
            }

            if from_contents.quantity * volume > to_inventory.pockets[to_index].volume {
                return Err("Target pocket can't hold that much".into());
            }

            let to_volume = if is_cargo {
                match ctx.db.cargo_desc().id().find(to_contents.item_id) {
                    Some(c) => c.volume,
                    None => return Err("Invalid target pocket".into()), // invalid cargo id, probably 0.
                }
            } else {
                match ctx.db.item_desc().id().find(to_contents.item_id) {
                    Some(i) => i.volume,
                    None => return Err("Invalid target pocket".into()), // invalid item id, probably 0.
                }
            };

            if to_contents.quantity * to_volume > from_inventory.pockets[from_index].volume {
                return Err("Source pocket can't hold that much".into());
            }

            //remove
            //Note: use set_at_and_discover to support item list movement
            InventoryState::set_at_and_discover(
                ctx,
                from_index,
                Some(to_contents),
                &mut discovery,
                &mut from_inventory,
                from_inventory_owner_entity_id == actor_id,
            );

            //add
            if same_inventory {
                //Note: use set_at_and_discover to support item list movement
                InventoryState::set_at_and_discover(
                    ctx,
                    to_index,
                    Some(item_stack),
                    &mut discovery,
                    &mut from_inventory,
                    from_inventory_owner_entity_id == actor_id,
                );
            } else {
                //Note: use set_at_and_discover to support item list movement
                InventoryState::set_at_and_discover(
                    ctx,
                    to_index,
                    Some(item_stack),
                    &mut discovery,
                    &mut to_inventory,
                    to_inventory_owner_entity_id == actor_id,
                );
            }
        } else {
            //remove
            //Note: use set_at_and_discover to support item list movement
            InventoryState::set_at_and_discover(
                ctx,
                from_index,
                Some(item_stack.clone_with_quantity(from_contents.quantity - inserted_quantity)),
                &mut discovery,
                &mut from_inventory,
                from_inventory_owner_entity_id == actor_id,
            );

            //add
            let modified_item_stack = item_stack.clone_with_quantity(target_pocket_quantity + inserted_quantity);

            if same_inventory {
                //Note: use set_at_and_discover to support item list movement
                InventoryState::set_at_and_discover(
                    ctx,
                    to_index,
                    Some(modified_item_stack),
                    &mut discovery,
                    &mut from_inventory,
                    from_inventory_owner_entity_id == actor_id,
                );
            } else {
                //Note: use set_at_and_discover to support item list movement
                InventoryState::set_at_and_discover(
                    ctx,
                    to_index,
                    Some(modified_item_stack),
                    &mut discovery,
                    &mut to_inventory,
                    to_inventory_owner_entity_id == actor_id,
                );
            }
        }
    }

    discovery.commit(ctx);

    if from_inventory_type == inventory_type::InventoryType::LootChest && from_inventory.is_empty() {
        loot_chest_helpers::on_item_taken_from_loot_chest(ctx, from_inventory.owner_entity_id, true)?;
    }

    if !same_inventory {
        ActionLogState::log_storage(
            ctx,
            &from_inventory,
            &to_inventory,
            actor_id,
            from_inventory_type,
            to_inventory_type,
            &item_stack.clone_with_quantity(inserted_quantity),
        );
    }

    if from_inventory_type == inventory_type::InventoryType::Dropped {
        let dropped_inventory = ctx
            .db
            .dropped_inventory_state()
            .entity_id()
            .find(from_inventory.owner_entity_id)
            .unwrap();
        dropped_inventory.on_inventory_updated(ctx, actor_id, from_inventory);
    } else {
        ctx.db.inventory_state().entity_id().update(from_inventory);
    }

    // Something was swapped into or from the toolbelt
    if to_toolbelt || from_toolbelt {
        // Make sure we have sufficient skill level to "equip" on the toolbelt
        if to_inventory.inventory_index == 1 && to_inventory.owner_entity_id == actor_id {
            validate_equip_on_toolbelt(ctx, actor_id, from_contents.item_id, to_index as i32)?;
        }

        // Need to update destination inventory early for abilities updates
        if !same_inventory {
            ctx.db.inventory_state().entity_id().update(to_inventory);
        }

        if from_toolbelt && to_toolbelt {
            return Err("This should have been filtered off earlier.".into());
            // This should not happen, it will be filtered
        }
        if from_toolbelt {
            if let Some(to_pocket_contents) = to_pocket.contents {
                // It's being swapped to another fitting weapon type (toolbelt => inventory)

                // Make sure we can also equip the tool being swapped
                validate_equip_on_toolbelt(ctx, actor_id, to_pocket_contents.item_id, from_index as i32)?;

                PlayerState::on_updated_toolbelt(ctx, actor_id, from_contents.item_id, to_pocket_contents.item_id);
            } else {
                // It's being swapped with an empty slot, therefore simply removed
                PlayerState::on_removed_from_toolbelt(ctx, actor_id, from_contents.item_id);
            }
        } else {
            if let Some(to_pocket_contents) = to_pocket.contents {
                // It's being swapped to another fitting weapon type (inventory => toolbelt)

                // Make sure we can also equip the tool being swapped
                validate_equip_on_toolbelt(ctx, actor_id, to_pocket_contents.item_id, to_index as i32)?;

                PlayerState::on_updated_toolbelt(ctx, actor_id, to_pocket_contents.item_id, from_contents.item_id);
            } else {
                // It's being swapped with an empty slot, therefore simply removed
                PlayerState::on_added_to_toolbelt(ctx, actor_id, from_contents.item_id);
            }
        }

        PlayerState::collect_stats(ctx, actor_id);
    } else {
        if !same_inventory {
            ctx.db.inventory_state().entity_id().update(to_inventory);
        }
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

fn validate_equip_on_toolbelt(ctx: &ReducerContext, actor_id: u64, item_id: i32, to_index: i32) -> Result<(), String> {
    let equipment_info = unwrap_or_err!(ctx.db.equipment_desc().item_id().find(item_id), "Item can't be moved in toolbelt");

    if let Some(level_req) = &equipment_info.level_requirement {
        if !PlayerState::meets_level_requirement(ctx, actor_id, level_req) {
            let mut skill_name = ctx.db.skill_desc().id().find(level_req.skill_id).unwrap().name;
            if skill_name == "ANY" {
                skill_name = "in any skill".into();
            }
            return Err(format!("You need to be level {{0}} {{1}}|~{0}|~{1}", level_req.level, skill_name));
        }
    }
    if !AchievementDesc::evaluate_achievements(ctx, actor_id, equipment_info.required_achievements) {
        return Err("You don't meet the achievement requirements to equip this item".into());
    }

    for knowledge_req in &equipment_info.required_knowledges {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *knowledge_req) {
            return Err("You don't meet the knowledge requirements to equip this item".into());
        }
    }

    // Validate item ending on the pocket (in case it was a swap)
    if to_index == ToolTypeDesc::get_combat_weapon_slot(ctx) {
        let weapon_info = ctx.db.weapon_desc().item_id().find(item_id);
        if let Some(info) = weapon_info {
            let weapon_type = ctx.db.weapon_type_desc().id().find(info.weapon_type).unwrap();
            if weapon_type.hunting {
                return Err("Hunting tools can't be equipped there.".into());
            }
        } else {
            return Err("Only weapons go on this pocket.".into());
        }
    } else {
        let tool_info = ctx.db.tool_desc().item_id().filter(item_id).next();
        if to_index != ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool_info.unwrap().tool_type) {
            return Err("This tool doesn't go on this pocket.".into());
        }
    }

    Ok(())
}
