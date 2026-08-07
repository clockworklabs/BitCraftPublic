use crate::messages::static_data::{EquipmentSlot, EquipmentSlotType, SkillType};

impl EquipmentSlot {
    pub fn item_id(&self) -> i32 {
        if let Some(item) = self.item {
            return item.item_id;
        }
        0
    }

    pub fn default_equipment_slots() -> Vec<EquipmentSlot> {
        Self::all_equipment_slots()
            .iter()
            .map(|slot| EquipmentSlot {
                item: None,
                primary: *slot,
            })
            .collect()
    }

    pub fn all_equipment_slots() -> &'static [EquipmentSlotType] {
        &[
            EquipmentSlotType::MainHand,
            EquipmentSlotType::OffHand,
            EquipmentSlotType::HeadArtifact,
            EquipmentSlotType::TorsoArtifact,
            EquipmentSlotType::HandArtifact,
            EquipmentSlotType::FeetArtifact,
            EquipmentSlotType::HeadClothing,
            EquipmentSlotType::TorsoClothing,
            EquipmentSlotType::HandClothing,
            EquipmentSlotType::BeltClothing,
            EquipmentSlotType::LegClothing,
            EquipmentSlotType::FeetClothing,
            EquipmentSlotType::ForestryCharm,
            EquipmentSlotType::ForestryInstrument,
            EquipmentSlotType::CarpentryCharm,
            EquipmentSlotType::CarpentryInstrument,
            EquipmentSlotType::MasonryCharm,
            EquipmentSlotType::MasonryInstrument,
            EquipmentSlotType::MiningCharm,
            EquipmentSlotType::MiningInstrument,
            EquipmentSlotType::SmithingCharm,
            EquipmentSlotType::SmithingInstrument,
            EquipmentSlotType::LeatherworkingCharm,
            EquipmentSlotType::LeatherworkingInstrument,
            EquipmentSlotType::HuntingCharm,
            EquipmentSlotType::HuntingInstrument,
            EquipmentSlotType::TailoringCharm,
            EquipmentSlotType::TailoringInstrument,
            EquipmentSlotType::FarmingCharm,
            EquipmentSlotType::FarmingInstrument,
            EquipmentSlotType::FishingCharm,
            EquipmentSlotType::FishingInstrument,
            EquipmentSlotType::ForagingCharm,
            EquipmentSlotType::ForagingInstrument,
            EquipmentSlotType::ScholarCharm,
            EquipmentSlotType::ScholarInstrument,
        ]
    }

    pub fn equipment_preset_slots() -> &'static [EquipmentSlotType] {
        &[
            EquipmentSlotType::MainHand,
            EquipmentSlotType::OffHand,
            EquipmentSlotType::TorsoArtifact,
            EquipmentSlotType::HandArtifact,
            EquipmentSlotType::FeetArtifact,
            EquipmentSlotType::HeadClothing,
            EquipmentSlotType::TorsoClothing,
            EquipmentSlotType::HandClothing,
            EquipmentSlotType::BeltClothing,
            EquipmentSlotType::LegClothing,
            EquipmentSlotType::FeetClothing,
        ]
    }

    pub fn charm_for_skill(skill_type: SkillType) -> Option<EquipmentSlotType> {
        Some(match skill_type {
            SkillType::Forestry => EquipmentSlotType::ForestryCharm,
            SkillType::Carpentry => EquipmentSlotType::CarpentryCharm,
            SkillType::Masonry => EquipmentSlotType::MasonryCharm,
            SkillType::Mining => EquipmentSlotType::MiningCharm,
            SkillType::Smithing => EquipmentSlotType::SmithingCharm,
            SkillType::Leatherworking => EquipmentSlotType::LeatherworkingCharm,
            SkillType::Hunting => EquipmentSlotType::HuntingCharm,
            SkillType::Tailoring => EquipmentSlotType::TailoringCharm,
            SkillType::Farming => EquipmentSlotType::FarmingCharm,
            SkillType::Fishing => EquipmentSlotType::FishingCharm,
            SkillType::Foraging => EquipmentSlotType::ForagingCharm,
            SkillType::Scholar => EquipmentSlotType::ScholarCharm,
            _ => return None,
        })
    }

    pub fn instrument_for_skill(skill_type: SkillType) -> Option<EquipmentSlotType> {
        Some(match skill_type {
            SkillType::Forestry => EquipmentSlotType::ForestryInstrument,
            SkillType::Carpentry => EquipmentSlotType::CarpentryInstrument,
            SkillType::Masonry => EquipmentSlotType::MasonryInstrument,
            SkillType::Mining => EquipmentSlotType::MiningInstrument,
            SkillType::Smithing => EquipmentSlotType::SmithingInstrument,
            SkillType::Leatherworking => EquipmentSlotType::LeatherworkingInstrument,
            SkillType::Hunting => EquipmentSlotType::HuntingInstrument,
            SkillType::Tailoring => EquipmentSlotType::TailoringInstrument,
            SkillType::Farming => EquipmentSlotType::FarmingInstrument,
            SkillType::Fishing => EquipmentSlotType::FishingInstrument,
            SkillType::Foraging => EquipmentSlotType::ForagingInstrument,
            SkillType::Scholar => EquipmentSlotType::ScholarInstrument,
            _ => return None,
        })
    }
}
