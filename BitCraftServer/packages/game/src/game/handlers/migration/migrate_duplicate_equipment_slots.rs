use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{equipment_preset_state, equipment_state, inventory_state},
        game_util::ItemStack,
        static_data::EquipmentSlot,
    },
    InventoryState, PlayerState,
};

fn repair_slots(equipment_slots: &[EquipmentSlot]) -> (Vec<EquipmentSlot>, Vec<ItemStack>, bool) {
    let expected_slots = EquipmentSlot::all_equipment_slots();
    let mut remaining_slots = equipment_slots.to_vec();
    let mut repaired_slots = Vec::with_capacity(expected_slots.len());
    let mut removed_items = Vec::new();

    // Rebuild the vector in canonical order. This repairs duplicate, missing, and
    // out-of-order entries without deleting valid slots that appear after a gap.
    for slot_type in expected_slots {
        if let Some(slot_index) = remaining_slots.iter().position(|slot| slot.primary == *slot_type) {
            repaired_slots.push(remaining_slots.remove(slot_index));
        } else {
            repaired_slots.push(EquipmentSlot {
                item: None,
                primary: *slot_type,
            });
        }
    }

    // All unmatched entries are duplicate or invalid slots. Return their contents
    // unless an identical multi-slot representation remains equipped.
    for removed_slot in remaining_slots {
        if let Some(item) = removed_slot.item {
            removed_items.push(item);
        }
    }

    // A multi-slot item is represented by an identical ItemStack in every occupied
    // slot. Do not return a copy that is still equipped, and return at most one copy
    // when every representation was removed.
    removed_items.retain(|item| !repaired_slots.iter().any(|slot| slot.item == Some(*item)));
    removed_items.dedup();

    let changed = repaired_slots != equipment_slots;
    (repaired_slots, removed_items, changed)
}

#[spacetimedb::reducer]
pub fn migrate_duplicate_equipment_slots(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut repaired_players = 0;
    let mut repaired_rows = 0;
    let mut returned_items = 0;
    let mut skipped_players = 0;

    for equipment_state in ctx.db.equipment_state().iter() {
        let player_entity_id = equipment_state.entity_id;
        let (repaired_base_slots, mut items_to_return, base_changed) = repair_slots(&equipment_state.equipment_slots);

        let mut repaired_presets = Vec::new();
        for preset in ctx.db.equipment_preset_state().player_entity_id().filter(player_entity_id) {
            let (repaired_slots, preset_items_to_return, changed) = repair_slots(&preset.equipment_slots);
            items_to_return.extend(preset_items_to_return);
            repaired_presets.push((preset, repaired_slots, changed));
        }

        let changed_rows = usize::from(base_changed) + repaired_presets.iter().filter(|(_, _, changed)| *changed).count();
        if changed_rows == 0 && items_to_return.is_empty() {
            continue;
        }

        let mut inventory = None;
        if !items_to_return.is_empty() {
            let Some(mut player_inventory) = InventoryState::get_player_inventory(ctx, player_entity_id) else {
                skipped_players += 1;
                log::warn!(
                    "Skipping equipment slot repair for player {}: no player inventory",
                    player_entity_id
                );
                continue;
            };

            if !player_inventory.fits_all(ctx, &items_to_return) {
                skipped_players += 1;
                log::warn!(
                    "Skipping equipment slot repair for player {}: {} recovered item stacks do not fit in inventory",
                    player_entity_id,
                    items_to_return.len()
                );
                continue;
            }

            for item in &items_to_return {
                debug_assert!(player_inventory.add(ctx, *item));
            }
            inventory = Some(player_inventory);
        }

        if base_changed {
            let mut repaired_state = equipment_state;
            repaired_state.equipment_slots = repaired_base_slots;
            ctx.db.equipment_state().entity_id().update(repaired_state);
        }

        for (mut preset, repaired_slots, changed) in repaired_presets {
            if changed {
                preset.equipment_slots = repaired_slots;
                ctx.db.equipment_preset_state().entity_id().update(preset);
            }
        }

        if let Some(inventory) = inventory {
            ctx.db.inventory_state().entity_id().update(inventory);
            returned_items += items_to_return.len();
        }

        PlayerState::collect_stats(ctx, player_entity_id);
        repaired_players += 1;
        repaired_rows += changed_rows;
    }

    log::info!(
        "Equipment slot repair: players={}, rows={}, returned_items={}, skipped_players={}",
        repaired_players,
        repaired_rows,
        returned_items,
        skipped_players,
    );

    Ok(())
}
