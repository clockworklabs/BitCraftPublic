use spacetimedb::ReducerContext;

use crate::messages::static_data::PavingTileDesc;
use crate::{character_stat_desc, CharacterStatType, CsvStatEntry};

impl PavingTileDesc {
    pub fn try_get_stat(&self, stat_type: CharacterStatType) -> Option<&CsvStatEntry> {
        assert!(
            match stat_type {
                CharacterStatType::MovementMultiplier => true,
                //CharacterStatType::SprintMultiplier => true,
                //CharacterStatType::SprintStaminaDrain => true,
                _ => false,
            },
            "Stat {{0}} is not supported in pavement|~{:?}",
            stat_type
        );

        self.stat_effects.iter().find(|stat| stat.id == stat_type)
    }

    pub fn apply_stat_to_value(&self, ctx: &ReducerContext, val: f32, stat_type: CharacterStatType) -> f32 {
        if let Some(stat) = self.try_get_stat(stat_type) {
            let desc = ctx.db.character_stat_desc().stat_type().find(stat_type as i32).unwrap();
            if stat.is_pct { val * (1.0 + stat.value) } else { val + stat.value }.clamp(desc.min_value, desc.max_value)
        } else {
            val
        }
    }

    pub fn apply_stat_to_value_unclamped(&self, val: f32, stat_type: CharacterStatType) -> f32 {
        if let Some(stat) = self.try_get_stat(stat_type) {
            if stat.is_pct { val * (1.0 + stat.value) } else { val + stat.value }
        } else {
            val
        }
    }
}
