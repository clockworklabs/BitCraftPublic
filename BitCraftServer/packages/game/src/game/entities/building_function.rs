use crate::{building_type_desc, messages::static_data::BuildingFunction, BuildingCategory, BuildingDesc};
use spacetimedb::ReducerContext;

impl BuildingFunction {
    pub fn has_category(&self, ctx: &ReducerContext, category: BuildingCategory) -> bool {
        if let Some(building_type) = ctx.db.building_type_desc().id().find(self.function_type) {
            return building_type.category == category;
        }

        false
    }

    pub fn has_inventory(&self) -> bool {
        self.cargo_slots > 0 || self.storage_slots > 0
    }

    pub fn has_progressive_action_state(&self) -> bool {
        self.refining_slots > 0 || self.refining_cargo_slots > 0 || self.crafting_slots > 0
    }

    pub fn get_inventory_index(function_type: i32, description: &BuildingDesc) -> Option<i32> {
        let mut inventory_index = -1;

        // Enumerate through functions to determine which ones own an inventory
        for f in &description.functions {
            if f.storage_slots > 0 || f.cargo_slots > 0 {
                inventory_index += 1;
                if function_type == f.function_type {
                    return Some(inventory_index);
                }
            }
        }
        None
    }

    pub fn from_inventory_index(description: &BuildingDesc, inventory_index: i32) -> Option<&BuildingFunction> {
        let mut cur_inventory_index = -1;

        // Enumerate through functions to determine which ones own an inventory
        // We cannot simply use inventory_index with functions vec as some building functions may not have inventories
        for f in &description.functions {
            if f.storage_slots > 0 || f.cargo_slots > 0 {
                cur_inventory_index += 1;
                if cur_inventory_index == inventory_index {
                    return Some(f);
                }
            }
        }
        None
    }

    pub fn max_concurrent_crafts(description: &BuildingDesc) -> i32 {
        for f in &description.functions {
            if f.concurrent_crafts_per_player > 0 {
                return f.concurrent_crafts_per_player;
            }
        }
        0
    }

    pub fn max_housing_slots(description: &BuildingDesc) -> i32 {
        for f in &description.functions {
            if f.housing_slots != 0 {
                return f.housing_slots;
            }
        }
        0
    }

    pub fn player_housing_income(description: &BuildingDesc) -> u32 {
        for f in &description.functions {
            if f.housing_income != 0 {
                return f.housing_income;
            }
        }
        0
    }
}
