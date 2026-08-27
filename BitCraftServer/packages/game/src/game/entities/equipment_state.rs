use std::collections::HashMap;

use spacetimedb::rand::Rng;
use spacetimedb::ReducerContext;

use crate::game::entities::buff;
use crate::messages::components::{equipment_preset_state, equipment_state, EquipmentState};
use crate::messages::static_data::*;
use crate::InventoryState;

impl EquipmentState {
    fn active_worn_equipment_item_ids(&self, ctx: &ReducerContext) -> Vec<i32> {
        let active_preset = ctx
            .db
            .equipment_preset_state()
            .player_entity_id()
            .filter(self.entity_id)
            .find(|preset| preset.active);

        let equipment_slots = if let Some(preset) = active_preset {
            EquipmentSlot::all_equipment_slots()
                .iter()
                .enumerate()
                .map(|(slot_index, slot_type)| {
                    if EquipmentSlot::equipment_preset_slots().contains(slot_type) {
                        preset.equipment_slots[slot_index].clone()
                    } else {
                        self.equipment_slots[slot_index].clone()
                    }
                })
                .collect()
        } else {
            self.equipment_slots.clone()
        };

        EquipmentSlot::all_equipment_slots()
            .iter()
            .zip(equipment_slots.iter())
            .filter_map(|(slot_type, equipment_slot)| {
                if equipment_slot.item_id() > 0 && equipment_slot.primary == *slot_type {
                    Some(equipment_slot.item_id())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn collect_stats(&self, ctx: &ReducerContext, bonuses: &mut HashMap<CharacterStatType, (f32, f32)>) {
        let active_preset = ctx
            .db
            .equipment_preset_state()
            .player_entity_id()
            .filter(self.entity_id)
            .find(|preset| preset.active);

        let equipment_slots = if let Some(preset) = active_preset {
            EquipmentSlot::all_equipment_slots()
                .iter()
                .enumerate()
                .map(|(slot_index, slot_type)| {
                    if EquipmentSlot::equipment_preset_slots().contains(slot_type) {
                        preset.equipment_slots[slot_index].clone()
                    } else {
                        self.equipment_slots[slot_index].clone()
                    }
                })
                .collect()
        } else {
            self.equipment_slots.clone()
        };

        // collect item ids from equipped gear (extra check to avoid doubling equipment taking 2 slots, although this is no longer used outside character customization)
        let mut equipped_item_ids: Vec<i32> = EquipmentSlot::all_equipment_slots()
            .iter()
            .zip(equipment_slots.iter())
            .filter_map(|(slot_type, equipment_slot)| {
                if equipment_slot.item_id() > 0 && equipment_slot.primary == *slot_type {
                    Some(equipment_slot.item_id())
                } else {
                    None
                }
            })
            .collect();

        // collect toolbelt item ids
        let toolbelt_inv = InventoryState::get_player_toolbelt(ctx, self.entity_id).unwrap();
        for p in toolbelt_inv.pockets {
            if let Some(content) = p.contents {
                equipped_item_ids.push(content.item_id);
            }
        }

        // apply all equipped item stats
        for item_id in equipped_item_ids {
            if let Some(equipment) = ctx.db.equipment_desc().item_id().find(&item_id) {
                for stat_delta in &equipment.stats {
                    let entry = bonuses.entry(stat_delta.id).or_insert((0.0, 0.0));
                    if stat_delta.is_pct {
                        *entry = (entry.0, entry.1 + stat_delta.value);
                    } else {
                        *entry = (entry.0 + stat_delta.value, entry.1);
                    }
                }
            }
        }
    }

    pub fn try_activate_profession_hit_buffs(
        ctx: &ReducerContext,
        player_entity_id: u64,
        action_skill: Option<SkillType>,
    ) -> Result<(), String> {
        let Some(equipment_state) = ctx.db.equipment_state().entity_id().find(&player_entity_id) else {
            return Ok(());
        };

        for item_id in equipment_state.active_worn_equipment_item_ids(ctx) {
            let Some(equipment) = ctx.db.equipment_desc().item_id().find(&item_id) else {
                continue;
            };

            if equipment.equipment_buff_id == 0 || equipment.equipment_buff_chance_per_hit <= 0.0 {
                continue;
            }

            if equipment.equipment_buff_skill_id != SkillType::ANY as i32 {
                let Some(action_skill) = action_skill else {
                    continue;
                };

                if equipment.equipment_buff_skill_id != action_skill as i32 {
                    continue;
                }
            }

            if ctx.rng().gen_range(0.0..=1.0) <= equipment.equipment_buff_chance_per_hit {
                buff::activate(ctx, player_entity_id, equipment.equipment_buff_id, None, None)?;
            }
        }

        Ok(())
    }

    /*
    pub fn get_weapon(&self, weapon_requirements: &Vec<WeaponRequirement>) -> Option<WeaponDesc> {
        let equipment_slots = &self.equipment_slots;
        let main_hand_slot = &equipment_slots[EquipmentSlotType::MainHand as usize];
        let off_hand_slot = &equipment_slots[EquipmentSlotType::OffHand as usize];
        for req in weapon_requirements {
            // check main hand slot
            if main_hand_slot.item_id() > 0 {
                if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&main_hand_slot.item_id()) {
                    if req.weapon_type == weapon.weapon_type {
                        return Some(weapon.clone());
                    }
                }
            }

            // check off-hand slot
            if off_hand_slot.item_id() > 0 {
                if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&off_hand_slot.item_id()) {
                    if req.weapon_type == weapon.weapon_type {
                        return Some(weapon.clone());
                    }
                }
            }
        }
        None
    }

    pub fn meet_requirement(&self, weapon_requirements: &Vec<WeaponRequirement>) -> bool {
        if self.get_weapon(weapon_requirements).is_some() {
            return true;
        }
        let equipment_slots = &self.equipment_slots;
        let main_hand_slot = &equipment_slots[EquipmentSlotType::MainHand as usize];
        main_hand_slot.item_id() == 0 && weapon_requirements.iter().any(|r| r.weapon_type == 0)
    }
    */
    /*
    pub fn meet_requirement(&self, weapon_requirements: &Vec<WeaponRequirement>) -> bool {
        if let Some(tool) = ToolDesc::get_best_weapon(self.entity_id) {
            if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&tool.item_id) {
                return weapon_requirements.iter().any(|r| r.weapon_type == weapon.weapon_type);
            }
        }
        if weapon_requirements.iter().all(|w| w.weapon_type == 0) {
            return true;
        }

        false
    }
    */
}
