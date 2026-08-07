use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{equipment_preset_state, equipment_state},
        static_data::EquipmentSlot,
    },
};

#[spacetimedb::reducer]
pub fn migrate_equipment_slots(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let default_slots = EquipmentSlot::default_equipment_slots();
    let expected_len = default_slots.len();

    let mut equipment_state_count = 0;
    let mut equipment_preset_state_count = 0;

    for mut equipment_state in ctx.db.equipment_state().iter() {
        if equipment_state.equipment_slots.len() >= expected_len {
            continue;
        }

        equipment_state
            .equipment_slots
            .extend(default_slots.iter().skip(equipment_state.equipment_slots.len()).cloned());
        ctx.db.equipment_state().entity_id().update(equipment_state);
        equipment_state_count += 1;
    }

    for mut equipment_preset_state in ctx.db.equipment_preset_state().iter() {
        if equipment_preset_state.equipment_slots.len() >= expected_len {
            continue;
        }

        equipment_preset_state
            .equipment_slots
            .extend(default_slots.iter().skip(equipment_preset_state.equipment_slots.len()).cloned());
        ctx.db.equipment_preset_state().entity_id().update(equipment_preset_state);
        equipment_preset_state_count += 1;
    }

    log::info!(
        "Migrated {} equipment_state rows and {} equipment_preset_state rows to {} equipment slots",
        equipment_state_count,
        equipment_preset_state_count,
        expected_len
    );

    Ok(())
}
