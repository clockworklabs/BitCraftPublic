use spacetimedb::rand::Rng;
use spacetimedb::{ReducerContext, Table};

use crate::character_stats_state;
use crate::messages::components::{CharacterStatsState, PlayerState};
use crate::messages::static_data::*;

impl CharacterStatType {
    pub fn to_enum(value: i32) -> CharacterStatType {
        unsafe { std::mem::transmute(value) }
    }
}

impl CharacterStatsState {
    pub fn new(ctx: &ReducerContext, entity_id: u64) -> CharacterStatsState {
        let types: Vec<i32> = ctx.db.character_stat_desc().iter().map(|cs| cs.stat_type).collect();
        let mut stats = CharacterStatsState {
            entity_id,
            values: vec![0.0; types.len()],
        };

        for t in types {
            let value = ctx.db.character_stat_desc().stat_type().find(t).unwrap().value;
            stats.set(CharacterStatType::to_enum(t), value);
        }
        stats
    }

    pub fn set(&mut self, type_index: CharacterStatType, value: f32) {
        let index = type_index as usize;
        if let Some(stat) = self.values.get_mut(index) {
            *stat = value;
        }
    }

    pub fn get(&self, type_index: CharacterStatType) -> f32 {
        let index = type_index as usize;
        let stat = self.values.get(index).unwrap_or(&0.0);
        *stat
    }

    pub fn get_skill_speed(&self, skill: SkillType) -> f32 {
        if let Some(stat) = Self::stat_for_skill_speed(skill) {
            return self.get(stat);
        }
        1f32
    }

    pub fn get_skill_power(&self, skill: SkillType) -> f32 {
        if let Some(stat) = Self::stat_for_skill_power(skill) {
            return self.get(stat);
        }
        0f32
    }

    pub fn get_skill_crit_chance(&self, skill: SkillType) -> f32 {
        if let Some(stat) = Self::stat_for_crit_chance(skill) {
            return self.get(stat);
        }
        0f32
    }

    pub fn get_skill_crit_multiplier(&self, skill: SkillType) -> f32 {
        if let Some(stat) = Self::stat_for_crit_multiplier(skill) {
            return 1.0 + self.get(stat);
        }
        1f32
    }

    pub fn get_final_crit_multiplier(&self, ctx: &ReducerContext, skill: Option<SkillType>) -> f32 {
        let crit_chance = match skill {
            Some(skill) => self.get_skill_crit_chance(skill),
            None => 0.0,
        };
        let crit_multiplier = match skill {
            Some(skill) => self.get_skill_crit_multiplier(skill),
            None => 1.0,
        };

        // crit_multiplier from now on is applied to the damage, whether the crit happened (previous value) or not (1.0)
        if ctx.rng().gen_range(0f32..=1f32) <= crit_chance {
            crit_multiplier
        } else {
            1.0
        }
    }

    pub fn equals(&self, other: &CharacterStatsState) -> bool {
        // the two CharacterStats should have the same length, if not there is a migration bug.
        for i in 0..self.values.len() {
            let new_value = self.values[i];
            if new_value != other.values[i] {
                return false;
            }
        }
        true
    }

    pub fn get_entity_stat(ctx: &ReducerContext, entity_id: u64, stat: CharacterStatType) -> f32 {
        if let Some(stats) = ctx.db.character_stats_state().entity_id().find(entity_id) {
            return stats.get(stat);
        }
        0.0
    }

    pub fn get_deployable_speed_multiplier(ctx: &ReducerContext, entity_id: u64, deployable_type: DeployableType) -> f32 {
        Self::stat_for_deployable_type(deployable_type)
            .map(|stat| Self::get_entity_stat(ctx, entity_id, stat))
            .unwrap_or(1.0)
    }

    pub fn stat_for_deployable_type(deployable_type: DeployableType) -> Option<CharacterStatType> {
        match deployable_type {
            DeployableType::Cart => Some(CharacterStatType::CartSpeed),
            DeployableType::Mount => Some(CharacterStatType::MountSpeed),
            DeployableType::Boat => Some(CharacterStatType::BoatSpeed),
            _ => None,
        }
    }

    fn stat_for_skill_speed(skill: SkillType) -> Option<CharacterStatType> {
        //DAB Note: this is temporary, see comment inside `SkillType` definition
        match skill {
            SkillType::Forestry => Some(CharacterStatType::ForestrySpeed),
            SkillType::Carpentry => Some(CharacterStatType::CarpentrySpeed),
            SkillType::Masonry => Some(CharacterStatType::MasonrySpeed),
            SkillType::Mining => Some(CharacterStatType::MiningSpeed),
            SkillType::Smithing => Some(CharacterStatType::SmithingSpeed),
            SkillType::Scholar => Some(CharacterStatType::ScholarSpeed),
            SkillType::Leatherworking => Some(CharacterStatType::LeatherworkingSpeed),
            SkillType::Hunting => Some(CharacterStatType::HuntingSpeed),
            SkillType::Tailoring => Some(CharacterStatType::TailoringSpeed),
            SkillType::Farming => Some(CharacterStatType::FarmingSpeed),
            SkillType::Fishing => Some(CharacterStatType::FishingSpeed),
            SkillType::Cooking => Some(CharacterStatType::CookingSpeed),
            SkillType::Foraging => Some(CharacterStatType::ForagingSpeed),
            SkillType::HexiteGathering => Some(CharacterStatType::HexiteGatheringSpeed),
            _ => None,
        }
    }

    fn stat_for_skill_power(skill: SkillType) -> Option<CharacterStatType> {
        //DAB Note: this is temporary, see comment inside `SkillType` definition
        match skill {
            SkillType::Forestry => Some(CharacterStatType::ForestryPower),
            SkillType::Carpentry => Some(CharacterStatType::CarpentryPower),
            SkillType::Masonry => Some(CharacterStatType::MasonryPower),
            SkillType::Mining => Some(CharacterStatType::MiningPower),
            SkillType::Smithing => Some(CharacterStatType::SmithingPower),
            SkillType::Scholar => Some(CharacterStatType::ScholarPower),
            SkillType::Leatherworking => Some(CharacterStatType::LeatherworkingPower),
            SkillType::Hunting => Some(CharacterStatType::HuntingPower),
            SkillType::Tailoring => Some(CharacterStatType::TailoringPower),
            SkillType::Farming => Some(CharacterStatType::FarmingPower),
            SkillType::Fishing => Some(CharacterStatType::FishingPower),
            SkillType::Cooking => Some(CharacterStatType::CookingPower),
            SkillType::Foraging => Some(CharacterStatType::ForagingPower),
            SkillType::Construction => Some(CharacterStatType::ConstructionPower),
            SkillType::HexiteGathering => Some(CharacterStatType::HexiteGatheringPower),
            _ => None,
        }
    }

    fn stat_for_crit_chance(skill: SkillType) -> Option<CharacterStatType> {
        //DAB Note: this is temporary, see comment inside `SkillType` definition
        match skill {
            SkillType::Forestry => Some(CharacterStatType::ForestryCritChance),
            SkillType::Carpentry => Some(CharacterStatType::CarpentryCritChance),
            SkillType::Masonry => Some(CharacterStatType::MasonryCritChance),
            SkillType::Mining => Some(CharacterStatType::MiningCritChance),
            SkillType::Smithing => Some(CharacterStatType::SmithingCritChance),
            SkillType::Scholar => Some(CharacterStatType::ScholarCritChance),
            SkillType::Leatherworking => Some(CharacterStatType::LeatherworkingCritChance),
            SkillType::Hunting => Some(CharacterStatType::HuntingCritChance),
            SkillType::Tailoring => Some(CharacterStatType::TailoringCritChance),
            SkillType::Farming => Some(CharacterStatType::FarmingCritChance),
            SkillType::Fishing => Some(CharacterStatType::FishingCritChance),
            SkillType::Cooking => None,
            SkillType::Foraging => Some(CharacterStatType::ForagingCritChance),
            SkillType::Construction => None,
            SkillType::HexiteGathering => Some(CharacterStatType::HexiteGatheringCritChance),
            _ => None,
        }
    }

    fn stat_for_crit_multiplier(skill: SkillType) -> Option<CharacterStatType> {
        //DAB Note: this is temporary, see comment inside `SkillType` definition
        match skill {
            SkillType::Forestry => Some(CharacterStatType::ForestryCritMultiplier),
            SkillType::Carpentry => Some(CharacterStatType::CarpentryCritMultiplier),
            SkillType::Masonry => Some(CharacterStatType::MasonryCritMultiplier),
            SkillType::Mining => Some(CharacterStatType::MiningCritMultiplier),
            SkillType::Smithing => Some(CharacterStatType::SmithingCritMultiplier),
            SkillType::Scholar => Some(CharacterStatType::ScholarCritMultiplier),
            SkillType::Leatherworking => Some(CharacterStatType::LeatherworkingCritMultiplier),
            SkillType::Hunting => Some(CharacterStatType::HuntingCritMultiplier),
            SkillType::Tailoring => Some(CharacterStatType::TailoringCritMultiplier),
            SkillType::Farming => Some(CharacterStatType::FarmingCritMultiplier),
            SkillType::Fishing => Some(CharacterStatType::FishingCritMultiplier),
            SkillType::Cooking => None,
            SkillType::Foraging => Some(CharacterStatType::ForagingCritMultiplier),
            SkillType::Construction => None,
            SkillType::HexiteGathering => Some(CharacterStatType::HexiteGatheringCritMultiplier),
            _ => None,
        }
    }

    pub fn get_cooldown_and_weapon_cooldown_multipliers(ctx: &ReducerContext, player_entity_id: u64, hunting_weapon: bool) -> (f32, f32) {
        let mut cooldown_multiplier = CharacterStatsState::get_entity_stat(ctx, player_entity_id, CharacterStatType::CooldownMultiplier);
        if cooldown_multiplier == 0.0 {
            cooldown_multiplier = 1.0;
        }

        let weapon_id = if hunting_weapon {
            PlayerState::get_hunting_weapon(ctx, player_entity_id).unwrap().item_id
        } else {
            PlayerState::get_combat_weapon(ctx, player_entity_id).unwrap().item_id
        };
        let weapon_cooldown_multiplier = ctx.db.weapon_desc().item_id().find(weapon_id).unwrap().cooldown;
        (cooldown_multiplier, weapon_cooldown_multiplier)
    }
}
