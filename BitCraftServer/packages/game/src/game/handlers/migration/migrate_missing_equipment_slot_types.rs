use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{equipment_preset_state, equipment_state},
        static_data::{EquipmentSlot, EquipmentSlotType},
    },
};

fn add_missing_equipment_slot_types(equipment_slots: &mut Vec<EquipmentSlot>) -> usize {
    let mut added = 0;

    for (slot_index, slot_type) in EquipmentSlot::all_equipment_slots().iter().enumerate() {
        if equipment_slots.iter().any(|slot| slot.primary == *slot_type) {
            continue;
        }

        let missing_slot = EquipmentSlot {
            item: None,
            primary: *slot_type,
        };

        if let Some(existing_slot) = equipment_slots.get_mut(slot_index) {
            if existing_slot.primary == EquipmentSlotType::None && existing_slot.item.is_none() {
                *existing_slot = missing_slot;
            } else {
                equipment_slots.insert(slot_index, missing_slot);
            }
        } else {
            equipment_slots.push(missing_slot);
        }

        added += 1;
    }

    added
}

#[spacetimedb::reducer]
pub fn migrate_missing_equipment_slot_types(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut equipment_state_rows = 0;
    let mut equipment_state_slots = 0;
    let mut equipment_preset_state_rows = 0;
    let mut equipment_preset_state_slots = 0;

    for mut equipment_state in ctx.db.equipment_state().iter() {
        let added = add_missing_equipment_slot_types(&mut equipment_state.equipment_slots);
        if added > 0 {
            ctx.db.equipment_state().entity_id().update(equipment_state);
            equipment_state_rows += 1;
            equipment_state_slots += added;
        }
    }

    for mut equipment_preset_state in ctx.db.equipment_preset_state().iter() {
        let added = add_missing_equipment_slot_types(&mut equipment_preset_state.equipment_slots);
        if added > 0 {
            ctx.db.equipment_preset_state().entity_id().update(equipment_preset_state);
            equipment_preset_state_rows += 1;
            equipment_preset_state_slots += added;
        }
    }

    log::info!(
        "Missing equipment slot type migration: equipment_state rows={}, slots={}; equipment_preset_state rows={}, slots={}",
        equipment_state_rows,
        equipment_state_slots,
        equipment_preset_state_rows,
        equipment_preset_state_slots,
    );

    Ok(())
}