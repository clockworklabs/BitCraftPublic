use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::game::game_state::game_state_filters;
use crate::game::handlers::inventory::inventory_helper;
use crate::messages::action_request::PlayerEquipmentRemoveRequest;
use crate::messages::components::*;
use crate::messages::static_data::*;
use crate::unwrap_or_err;
use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn equipment_remove(ctx: &ReducerContext, request: PlayerEquipmentRemoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let remove_slot_index = request.slot as usize;
    if remove_slot_index == EquipmentSlotType::MainHand as usize || remove_slot_index == EquipmentSlotType::OffHand as usize {
        return Err("Obsolete - you can no longer equip main hand and offhand tools".into());
    }

    let slot_type = EquipmentSlot::all_equipment_slots()
        .get(request.slot as usize)
        .copied()
        .unwrap_or(EquipmentSlotType::None);
    let preset_index = if EquipmentSlot::equipment_preset_slots().contains(&slot_type) {
        request.preset_index
    } else {
        0
    };

    let mut equipment = ctx.db.equipment_state().entity_id().find(&actor_id).unwrap();
    let mut equipment_preset = None;
    let mut equipment_slots = &mut equipment.equipment_slots;

    if preset_index != 0 {
        equipment_preset = ctx
            .db
            .equipment_preset_state()
            .player_and_index()
            .filter((actor_id, preset_index))
            .next();
        if equipment_preset.is_none() {
            return Err("Invalid equipment preset index".into());
        }
        equipment_slots = &mut equipment_preset.as_mut().unwrap().equipment_slots;
    }

    let equipment_slot = &equipment_slots[remove_slot_index];
    if equipment_slot.item.is_none() {
        return Err("Nothing to remove".into());
    }

    let to_remove = unwrap_or_err!(equipment_slot.item, "No equipment in this slot.");

    // Reset all slots taken up by this item
    let remove_slot = equipment_slot.primary;
    for slot in equipment_slots.iter_mut() {
        if slot.primary == remove_slot {
            slot.item = None;
        }
    }

    if let Some(to_pocket) = &request.to_pocket {
        let pocket_index = to_pocket.pocket_index as usize;
        let mut inventory = unwrap_or_err!(
            ctx.db.inventory_state().entity_id().find(&to_pocket.inventory_entity_id),
            "Invalid target inventory"
        );

        let location = game_state_filters::coordinates_any(ctx, actor_id);

        // Make sure player can currently interact with specified inventory
        inventory_helper::validate_interact(ctx, actor_id, location, inventory.owner_entity_id, inventory.player_owner_entity_id)?;

        if inventory.inventory_index > 1 {
            return Err("Invalid target pocket".into());
        }

        if !inventory.is_pocket_empty(pocket_index) {
            let contents = inventory.get_pocket_contents(pocket_index).unwrap();
            let item_id = contents.item_id;
            if item_id <= 0 {
                return Err("Nothing to equip".into());
            }
            if contents.quantity > 1 {
                return Err("Can't swap".into());
            }

            let equipment_info = unwrap_or_err!(ctx.db.equipment_desc().item_id().find(&item_id), "Can't swap");

            if let Some(level_req) = &equipment_info.level_requirement {
                if !PlayerState::meets_level_requirement(ctx, actor_id, level_req) {
                    let mut skill_name = ctx.db.skill_desc().id().find(&level_req.skill_id).unwrap().name;
                    if skill_name == "ANY" {
                        skill_name = "in any skill".into();
                    }
                    return Err(format!("You need to be level {{0}} {{1}}|~{0}|~{1}", level_req.level, skill_name).into());
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

            for slot in equipment_info.slots.iter() {
                let slot = *slot as usize;
                let equipment_slot = &equipment_slots[slot];
                if equipment_slot.item_id() > 0 {
                    return Err("Can't swap".into());
                }
                equipment_slots[slot] = EquipmentSlot {
                    item: Some(contents),
                    primary: equipment_info.slots[0],
                };
            }

            inventory.remove_at(pocket_index);
        }

        if !inventory.add_at(ctx, pocket_index, to_remove) {
            return Err("Equipment won't fit in inventory".into());
        }
        ctx.db.inventory_state().entity_id().update(inventory);
    } else {
        // Default - no pocket means add to player inventory at first free slot.
        let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
        if !inventory.add(ctx, to_remove) {
            return Err("Equipment won't fit in inventory".into());
        }
        ctx.db.inventory_state().entity_id().update(inventory);
    }

    if let Some(preset) = equipment_preset {
        ctx.db.equipment_preset_state().entity_id().update(preset);
    } else {
        ctx.db.equipment_state().entity_id().update(equipment);
    }

    PlayerState::collect_stats(ctx, actor_id);

    Ok(())
}
