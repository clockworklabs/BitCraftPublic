use bitcraft_macro::feature_gate;
use std::collections::HashSet;

use crate::game::discovery::Discovery;
use crate::game::game_state::{self, game_state_filters};
use crate::game::handlers::inventory::inventory_helper;
use crate::messages::action_request::PlayerEquipmentAddRequest;
use crate::messages::components::*;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::static_data::*;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn equipment_add(ctx: &ReducerContext, request: PlayerEquipmentAddRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let from_pocket = unwrap_or_err!(&request.from_pocket, "Invalid origin pocket");
    let mut inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(&from_pocket.inventory_entity_id),
        "Invalid inventory"
    );

    let location = game_state_filters::coordinates_any(ctx, actor_id);

    // Make sure player can currently interact with specified inventory
    inventory_helper::validate_interact(ctx, actor_id, location, inventory.owner_entity_id, inventory.player_owner_entity_id)?;

    if inventory.is_pocket_cargo(from_pocket.pocket_index as usize) {
        return Err("Invalid origin pocket".into());
    }

    let pocket_index = from_pocket.pocket_index as usize;

    let contents = unwrap_or_err!(inventory.get_pocket_contents(pocket_index), "Invalid pocket");
    let item_id = contents.item_id;
    if item_id <= 0 {
        return Err("Nothing to equip".into());
    }

    // Only remove ONE item if the source is a stack since we can only equip one item per slot
    inventory.remove_quantity_at(pocket_index, 1);

    let equipment_info = unwrap_or_err!(ctx.db.equipment_desc().item_id().find(&item_id), "Item is not equipable");

    let preset_index = if EquipmentSlot::equipment_preset_slots().contains(&equipment_info.slots[0]) {
        request.preset_index
    } else {
        0
    };

    if equipment_info.slots[0] == EquipmentSlotType::MainHand || equipment_info.slots[0] == EquipmentSlotType::OffHand {
        return Err("Obsolete - you can no longer equip main hand and offhand tools".into());
    }

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

    let mut to_remove: HashSet<i32> = HashSet::new();
    for slot in equipment_info.slots.iter() {
        let equipment_slot = unwrap_or_err!(
            equipment_slots.iter_mut().find(|x| x.primary == *slot),
            "Unknown equipment slot '{{0}}'|~{:?}",
            *slot
        );
        if equipment_slot.item_id() > 0 {
            to_remove.insert(equipment_slot.item_id());
        }
        // Only equip ONE item if the source is a stack
        equipment_slot.item = Some(ItemStack::new(ctx, item_id, ItemType::Item, 1));
    }

    for id in to_remove.iter() {
        let item_stack = ItemStack::new(ctx, *id, ItemType::Item, 1);

        if !inventory.add_at(ctx, pocket_index, item_stack) && !inventory.add(ctx, item_stack) {
            return Err("Equipment that will be removed won't fit in inventory.".into());
        }
    }

    if to_remove.len() > 0 {
        for slot in equipment_slots.iter_mut() {
            if to_remove.contains(&slot.item_id()) && slot.item_id() != item_id {
                slot.item = None;
            }
        }
    }

    ctx.db.inventory_state().entity_id().update(inventory);
    if let Some(preset) = equipment_preset {
        ctx.db.equipment_preset_state().entity_id().update(preset);
    } else {
        ctx.db.equipment_state().entity_id().update(equipment);
    }
    PlayerState::collect_stats(ctx, actor_id);

    Ok(())
}
