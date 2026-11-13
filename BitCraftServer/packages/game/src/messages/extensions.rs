use spacetimedb::{ReducerContext, Table};

use super::{
    components::{Biome, Permission, PermissionGroup},
    generic::{
        globals, region_exploration_info, region_sign_in_parameters, world_region_state, RegionExplorationInfo, RegionSignInParameters,
    },
    static_data::*,
};

impl BuildingInteractionLevel {
    pub fn to_enum(value: i32) -> BuildingInteractionLevel {
        unsafe { std::mem::transmute(value) }
    }
}

impl FootprintType {
    pub fn to_enum(value: i32) -> FootprintType {
        unsafe { std::mem::transmute(value) }
    }
}

impl InteriorSpawnType {
    pub fn to_enum(value: i32) -> InteriorSpawnType {
        unsafe { std::mem::transmute(value) }
    }
}

impl MovementType {
    pub fn to_enum(value: i32) -> MovementType {
        unsafe { std::mem::transmute(value) }
    }
}

impl BuffTypeDesc {
    pub fn filter_by_category<'a>(ctx: &'a ReducerContext, category: BuffCategory) -> impl Iterator<Item = Self> + 'a {
        let category = category as i32;
        ctx.db.buff_type_desc().iter().filter(move |a| a.category == category)
    }
}

impl NpcType {
    pub fn to_enum(value: i32) -> NpcType {
        unsafe { std::mem::transmute(value) }
    }
}

impl Permission {
    pub fn to_enum(value: i32) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl PermissionGroup {
    pub fn to_enum(value: i32) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl BuffCategory {
    pub fn has_only_one_buff(self) -> bool {
        !matches!(self, BuffCategory::None | BuffCategory::Generic)
    }
    pub fn to_enum(value: i32) -> BuffCategory {
        unsafe { std::mem::transmute(value) }
    }
}

impl BuffDesc {
    pub fn filter_by_buff_category<'a>(ctx: &'a ReducerContext, category: BuffCategory) -> impl Iterator<Item = Self> + 'a {
        let category = category as i32;
        ctx.db
            .buff_type_desc()
            .category()
            .filter(category)
            .map(|bt| bt.id)
            .flat_map(|t| ctx.db.buff_desc().buff_type_id().filter(t))
    }

    pub fn find_by_buff_category_single(ctx: &ReducerContext, category: BuffCategory) -> Option<Self> {
        if !category.has_only_one_buff() {
            panic!("Use `filter_by_buff_category` to filter by generic categories");
        }
        let category = category as i32;
        ctx.db
            .buff_type_desc()
            .category()
            .filter(category)
            .map(|bt| bt.id)
            .flat_map(|t| ctx.db.buff_desc().buff_type_id().filter(t))
            .next()
    }
}

impl EnemyAiParamsDesc {
    pub fn filter_by_enemy_type<'a>(ctx: &'a ReducerContext, enemy_type: &'a EnemyType) -> impl Iterator<Item = Self> + 'a {
        ctx.db.enemy_ai_params_desc().iter().filter(|a| a.enemy_type == *enemy_type)
    }
}

impl BuildingCategory {
    pub fn to_enum(value: i32) -> BuildingCategory {
        unsafe { std::mem::transmute(value) }
    }
}

impl Biome {
    pub fn to_enum(value: u8) -> Biome {
        unsafe { std::mem::transmute(value) }
    }
}

impl SkillType {
    pub fn to_enum(value: i32) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl ExtractionRecipeDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl CraftingRecipeDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl ConstructionRecipeDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl DeconstructionRecipeDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl ResourcePlacementRecipeDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl PlaceableInteractionDesc {
    pub fn get_skill_type(&self) -> Option<SkillType> {
        let skill_id = match self.experience_per_progress.first() {
            Some(exp) => exp.skill_id,
            None => 0,
        };
        if skill_id > SkillType::ANY as i32 {
            return Some(SkillType::to_enum(skill_id));
        }
        None
    }
}

impl EmpireNotificationType {
    pub fn from_i32(value: i32) -> EmpireNotificationType {
        match value {
            1 => EmpireNotificationType::NewMember,
            2 => EmpireNotificationType::MarkedForSiege,
            3 => EmpireNotificationType::StartedSiege,
            4 => EmpireNotificationType::StartedDefense,
            5 => EmpireNotificationType::SuccessfulSiege,
            6 => EmpireNotificationType::SuccessfulDefense,
            7 => EmpireNotificationType::FailedSiege,
            8 => EmpireNotificationType::FailedDefense,
            9 => EmpireNotificationType::MemberLeft,
            10 => EmpireNotificationType::WatchtowerBuilt,
            11 => EmpireNotificationType::ClaimJoined,
            12 => EmpireNotificationType::ClaimLeft,
            13 => EmpireNotificationType::Donation,
            14 => EmpireNotificationType::DonationByProxy,
            _ => panic!("Unknown value: {}", value),
        }
    }
}

impl RegionSignInParameters {
    pub fn get(ctx: &ReducerContext) -> Option<RegionSignInParameters> {
        if let Some(globals) = ctx.db.globals().version().find(0) {
            return ctx.db.region_sign_in_parameters().region_id().find(globals.region_index);
        }

        None
    }
}

impl RegionExplorationInfo {
    pub fn counts_toward_achievements(ctx: &ReducerContext, region_id: u8) -> bool {
        ctx.db
            .region_exploration_info()
            .region_id()
            .find(region_id)
            .map(|info| info.counts_toward_achievements)
            .unwrap_or(true)
    }

    pub fn world_achievement_chunk_count(ctx: &ReducerContext) -> i32 {
        let region = ctx.db.world_region_state().id().find(0).unwrap();
        let chunks_per_region = region.region_width_chunks as i32 * region.region_height_chunks as i32;
        let excluded_region_count = ctx
            .db
            .region_exploration_info()
            .iter()
            .filter(|info| !info.counts_toward_achievements && info.region_id >= 1 && info.region_id <= region.region_count)
            .count() as i32;
        let eligible_region_count = region.region_count as i32 - excluded_region_count;

        eligible_region_count * chunks_per_region
    }
}
