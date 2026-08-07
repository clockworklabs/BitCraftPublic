use crate::messages::components::EquipmentPresetState;

use crate::game::game_state::{self};
use crate::messages::components::*;
use crate::messages::static_data::{equipment_preset_knowledge_desc, EquipmentSlot};
use spacetimedb::{ReducerContext, Table};

impl EquipmentPresetState {
    // No longer a reducer
    pub fn on_knowledge_acquired(ctx: &ReducerContext, player_entity_id: u64, knowledge_id: i32) {
        if ctx.db.equipment_preset_knowledge_desc().knowledge_id().find(knowledge_id).is_some() {
            let nb_equipment_presets = ctx.db.equipment_preset_state().player_entity_id().filter(player_entity_id).count();

            // Add equipment presets for each unmapped knowledge
            let equipment_slots = EquipmentSlot::default_equipment_slots();
            let preset = EquipmentPresetState {
                entity_id: game_state::create_entity(ctx),
                player_entity_id,
                index: nb_equipment_presets as i32 + 1,
                active: false,
                equipment_slots,
            };
            ctx.db.equipment_preset_state().insert(preset);
        }
    }
}
