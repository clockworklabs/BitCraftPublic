use crate::BuildingDesc;

impl BuildingDesc {
    pub fn crafting_slots(&self) -> i32 {
        for function in &self.functions {
            if function.crafting_slots > 0 {
                return function.crafting_slots;
            }
        }
        0
    }

    pub fn concurrent_slots(&self) -> i32 {
        for function in &self.functions {
            if function.crafting_slots > 0 {
                return function.concurrent_crafts_per_player;
            }
        }
        0
    }
}
