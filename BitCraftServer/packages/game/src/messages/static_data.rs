use bitcraft_macro::static_data_staging_table;
use spacetimedb::SpacetimeType;
use std::fmt;

use crate::messages::components::{AbilityType};
use crate::messages::game_util::{
    BuildingRequirement, ExperienceStackF32, InputItemStack, ItemListPossibility, ItemStack, LevelRequirement, CappedLevelRequirement, ProbabilisticItemStack,
    ToolRequirement,
};
use crate::messages::generic::PremiumServiceType;
use crate::PlayerActionLayer;
use crate::messages::util::MovementSpeed;

use super::components::Biome;
use super::game_util::DimensionType;

#[macro_export]
macro_rules! params {
    ($ctx:expr) => {
        $ctx.db.parameters_desc_v2().version().find(0).unwrap()
    };
}

#[spacetimedb::table(name = staged_static_data)]
pub struct StagedStaticData {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUpload,
}

#[derive(SpacetimeType, Default)]
pub struct StaticDataUpload {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDesc>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDesc>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
}

#[spacetimedb::table(name = staged_static_data_v2)]
pub struct StagedStaticDataV2 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV2,
}

#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV2 {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDesc>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV2>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDesc>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDesc>,
}


#[spacetimedb::table(name = staged_static_data_v3)]
pub struct StagedStaticDataV3 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV3,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV3 {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDesc>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV2>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDesc>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
}

#[spacetimedb::table(name = staged_static_data_v4)]
pub struct StagedStaticDataV4 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV4,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV4 {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDescV2>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV3>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDesc>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
    pub enemy_scaling: Vec<EnemyScalingDesc>,
}


#[spacetimedb::table(name = staged_static_data_v5)]
pub struct StagedStaticDataV5 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV5,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV5 {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDescV2>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV3>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDescV2>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
    pub wind_params: Vec<WindParamsDesc>,
    pub enemy_scaling: Vec<EnemyScalingDesc>,
}

#[spacetimedb::table(name = staged_static_data_v6)]
pub struct StagedStaticDataV6 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV6,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV6 {
    pub parameters: Vec<ParametersDesc>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDescV2>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV3>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDescV2>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
    pub wind_params: Vec<WindParamsDesc>,
    pub enemy_scaling: Vec<EnemyScalingDesc>,
}

#[spacetimedb::table(name = staged_static_data_v7)]
pub struct StagedStaticDataV7 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV7,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV7 {
    pub parameters: Vec<ParametersDescV2>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDescV2>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV3>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDescV2>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
    pub wind_params: Vec<WindParamsDesc>,
    pub enemy_scaling: Vec<EnemyScalingDesc>,
}

#[spacetimedb::table(name = staged_static_data_v8)]
pub struct StagedStaticDataV8 {
    #[primary_key]
    pub version: u32,
    pub static_data: StaticDataUploadV8,
}


#[derive(SpacetimeType, Default)]
pub struct StaticDataUploadV8 {
    pub parameters: Vec<ParametersDescV2>,
    pub private_parameters: Vec<PrivateParametersDesc>,
    pub secondary_knowledges: Vec<SecondaryKnowledgeDesc>,
    pub weapon_types: Vec<WeaponTypeDesc>,
    pub skills: Vec<SkillDesc>,
    pub targeting_matrices: Vec<TargetingMatrixDesc>,
    pub npcs: Vec<NpcDesc>,
    pub loot_rarities: Vec<LootRarityDesc>,
    pub knowledge_scroll_types: Vec<KnowledgeScrollTypeDesc>,
    pub items: Vec<ItemDesc>,
    pub enemy_ai_params: Vec<EnemyAiParamsDesc>,
    pub empire_ranks: Vec<EmpireRankDesc>,
    pub emotes: Vec<EmoteDesc>,
    pub climb_requirements: Vec<ClimbRequirementDesc>,
    pub claim_tile_costs: Vec<ClaimTileCost>,
    pub claim_tech: Vec<ClaimTechDesc>,
    pub character_stats: Vec<CharacterStatDesc>,
    pub cargos: Vec<CargoDesc>,
    pub building_types: Vec<BuildingTypeDesc>,
    pub buildings: Vec<BuildingDesc>,
    pub biomes: Vec<BiomeDesc>,
    pub interior_shapes: Vec<InteriorShapeDesc>,
    pub buff_types: Vec<BuffTypeDesc>,
    pub buffs: Vec<BuffDesc>,
    pub alerts: Vec<AlertDesc>,
    pub tool_types: Vec<ToolTypeDesc>,
    pub item_lists: Vec<ItemListDesc>,
    pub foods: Vec<FoodDesc>,
    pub achievements: Vec<AchievementDesc>,
    pub knowledge_stat_modifiers: Vec<KnowledgeStatModifierDesc>,
    pub interior_instances: Vec<InteriorInstanceDesc>,
    pub interior_environments: Vec<InteriorEnvironmentDesc>,
    pub interior_spawns: Vec<InteriorSpawnDesc>,
    pub building_portals: Vec<BuildingPortalDescV2>,
    pub portal_connections: Vec<InteriorPortalConnectionsDesc>,
    pub building_interior_networks: Vec<InteriorNetworkDesc>,
    pub building_claims: Vec<BuildingClaimDesc>,
    pub building_repairs: Vec<BuildingRepairsDesc>,
    pub building_spawns: Vec<BuildingSpawnDesc>,
    pub chest_rarities: Vec<ChestRarityDesc>,
    pub clothing: Vec<ClothingDesc>,
    pub collectibles: Vec<CollectibleDesc>,
    pub combat_actions: Vec<CombatActionDescV3>,
    pub combat_actions_multi_hit: Vec<CombatActionMultiHitDesc>,
    pub construction: Vec<ConstructionRecipeDesc>,
    pub crafting: Vec<CraftingRecipeDesc>,
    pub desconstructions: Vec<DeconstructionRecipeDesc>,
    pub empire_supplies: Vec<EmpireSuppliesDesc>,
    pub enemies: Vec<EnemyDesc>,
    pub environment_debuffs: Vec<EnvironmentDebuffDesc>,
    pub equipment: Vec<EquipmentDesc>,
    pub extraction: Vec<ExtractionRecipeDesc>,
    pub item_conversion_recipes: Vec<ItemConversionRecipeDesc>,
    pub knowledge_scrolls: Vec<KnowledgeScrollDesc>,
    pub loot_chests: Vec<LootChestDesc>,
    pub loot_tables: Vec<LootTableDesc>,
    pub paving: Vec<PavingTileDesc>,
    pub pathfinding: Vec<PathfindingDesc>,
    pub pillar_shaping: Vec<PillarShapingDesc>,
    pub resources: Vec<ResourceDesc>,
    pub resource_clumps: Vec<ResourceClumpDesc>,
    pub resource_growths: Vec<ResourceGrowthRecipeDesc>,
    pub resource_placements: Vec<ResourcePlacementRecipeDesc>,
    pub teleport_items: Vec<TeleportItemDesc>,
    pub tools: Vec<ToolDesc>,
    pub traveler_tasks: Vec<TravelerTaskDesc>,
    pub traveler_trade_orders: Vec<TravelerTradeOrderDesc>,
    pub deployables: Vec<DeployableDescV3>,
    pub weapons: Vec<WeaponDesc>,
    pub onboarding_rewards: Vec<OnboardingRewardDesc>,
    pub terraform_recipes: Vec<TerraformRecipeDesc>,
    pub empire_notifications: Vec<EmpireNotificationDesc>,
    pub empire_territories: Vec<EmpireTerritoryDesc>,
    pub empire_colors: Vec<EmpireColorDesc>,
    pub empire_icons: Vec<EmpireIconDesc>,
    pub walls: Vec<WallDesc>,
    pub gates: Vec<GateDesc>,
    pub elevators: Vec<ElevatorDesc>,
    pub player_actions: Vec<PlayerActionDesc>,
    pub distant_visibile_entities: Vec<DistantVisibleEntityDesc>,
    pub player_housing: Vec<PlayerHousingDesc>,
    pub hexite_exchange_entries: Vec<HexiteExchangeEntryDesc>,
    pub reserved_name: Vec<ReservedNameDesc>,
    pub contribution_loot: Vec<ContributionLootDescV2>,
    pub wind_params: Vec<WindParamsDesc>,
    pub enemy_scaling: Vec<EnemyScalingDesc>,
}

impl StaticDataUploadV8 {
    pub fn new() -> StaticDataUploadV8 {
        StaticDataUploadV8 {
            parameters: Vec::new(),
            private_parameters: Vec::new(),
            secondary_knowledges: Vec::new(),
            weapon_types: Vec::new(),
            skills: Vec::new(),
            targeting_matrices: Vec::new(),
            npcs: Vec::new(),
            loot_rarities: Vec::new(),
            knowledge_scroll_types: Vec::new(),
            items: Vec::new(),
            enemy_ai_params: Vec::new(),
            empire_ranks: Vec::new(),
            emotes: Vec::new(),
            climb_requirements: Vec::new(),
            claim_tile_costs: Vec::new(),
            claim_tech: Vec::new(),
            character_stats: Vec::new(),
            cargos: Vec::new(),
            building_types: Vec::new(),
            buildings: Vec::new(),
            biomes: Vec::new(),
            interior_shapes: Vec::new(),
            buff_types: Vec::new(),
            buffs: Vec::new(),
            alerts: Vec::new(),
            tool_types: Vec::new(),
            item_lists: Vec::new(),
            foods: Vec::new(),
            achievements: Vec::new(),
            knowledge_stat_modifiers: Vec::new(),
            interior_instances: Vec::new(),
            interior_environments: Vec::new(),
            interior_spawns: Vec::new(),
            building_portals: Vec::new(),
            portal_connections: Vec::new(),
            building_interior_networks: Vec::new(),
            building_claims: Vec::new(),
            building_repairs: Vec::new(),
            building_spawns: Vec::new(),
            chest_rarities: Vec::new(),
            clothing: Vec::new(),
            collectibles: Vec::new(),
            combat_actions: Vec::new(),
            combat_actions_multi_hit: Vec::new(),
            construction: Vec::new(),
            crafting: Vec::new(),
            desconstructions: Vec::new(),
            empire_supplies: Vec::new(),
            enemies: Vec::new(),
            environment_debuffs: Vec::new(),
            equipment: Vec::new(),
            extraction: Vec::new(),
            item_conversion_recipes: Vec::new(),
            knowledge_scrolls: Vec::new(),
            loot_chests: Vec::new(),
            loot_tables: Vec::new(),
            pathfinding: Vec::new(),
            paving: Vec::new(),
            pillar_shaping: Vec::new(),
            resources: Vec::new(),
            resource_clumps: Vec::new(),
            resource_growths: Vec::new(),
            resource_placements: Vec::new(),
            teleport_items: Vec::new(),
            tools: Vec::new(),
            traveler_tasks: Vec::new(),
            traveler_trade_orders: Vec::new(),
            deployables: Vec::new(),
            weapons: Vec::new(),
            onboarding_rewards: Vec::new(),
            terraform_recipes: Vec::new(),
            empire_notifications: Vec::new(),
            empire_territories: Vec::new(),
            empire_colors: Vec::new(),
            empire_icons: Vec::new(),
            walls: Vec::new(),
            gates: Vec::new(),
            elevators: Vec::new(),
            player_actions: Vec::new(),
            distant_visibile_entities: Vec::new(),
            player_housing: Vec::new(),
            hexite_exchange_entries: Vec::new(),
            reserved_name: Vec::new(),
            contribution_loot: Vec::new(),
            wind_params: Vec::new(),
            enemy_scaling: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "BuildingCategory")]
#[repr(i32)]
pub enum BuildingCategory {
    Storage,
    Crafting,
    Residential,
    TownHall,
    Wall,
    TradingPost,
    Ornamental,
    AncientRuins,
    ClaimTotem,
    TerrraformingBase,
    Barter,
    Portal,
    RentTerminal,
    Watchtower,
    EmpireFoundry,
    Sign,
    Gate,
    Bed,
    Waystone,
    Bank,
    Elevator,
    TownMarket,
    RecoveryChest,
    PlayerHousing,
    PremiumBuilding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "BuildingInteractionLevel")]
#[repr(i32)]
pub enum BuildingInteractionLevel {
    None,
    Claim,
    Empire,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "TraversalSettings")]
#[repr(i32)]
pub enum PathfindingTraversalSettings {
    None,
    FinishMove,
    Walk,
    StepUp,
    Hoist1,
    Down1,
    WalkSwimTransition,
    SwimHoist,
    Down1Swim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "CollectibleType")]
pub enum CollectibleType {
    Default,
    Hair,
    Mask,
    MaskPattern,
    HairColor,
    Nameplate,
    BodyColor,
    Emblem,
    ClothesHead,
    ClothesBelt,
    ClothesTorso,
    ClothesArms,
    ClothesLegs,
    ClothesFeet,
    Deployable,
    Title,
    Crown,
    Pet,
    ClothesCape,
    PremiumItem,
    Emote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "DeployableType")]
pub enum DeployableType {
    Cart,
    Mount,
    Stall,
    Cache,
    Boat,
    SiegeEngine,
}

impl fmt::Display for DeployableType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "ClothingMask")]
pub enum ClothingMask {
    None,
    HairFront,
    HairBottom,
    HairFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "EquipmentSlotType")]
#[repr(u8)]
pub enum EquipmentSlotType {
    MainHand,
    OffHand,
    HeadArtifact,
    TorsoArtifact,
    HandArtifact,
    FeetArtifact,
    HeadClothing,
    TorsoClothing,
    HandClothing,
    BeltClothing,
    LegClothing,
    FeetClothing,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "EquipmentVisualType")]
#[repr(i32)]
pub enum EquipmentVisualType {
    MainHand,
    OffHand,
    Clothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "BuffCategory")]
#[repr(i32)]
pub enum BuffCategory {
    None,
    Generic,
    Rested,
    RezSicknessShortTerm,
    RezSicknessLongTerm,
    InnerLight,
    Darkness,
    CarryCargo,
    Freezing,
    Overheating,
    FriendWarpDebuff,
    Starving,
    ElevatorSickness,
    NearbyFlame,
    Sprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "EnvironmentResistanceType")]
pub enum EnvironmentResistanceType {
    Cold,
    Heat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "EnemyType")]
#[repr(i32)]
pub enum EnemyType {
    None = 0,
    PracticeDummy = 1,
    
    // Huntable Animals
    GrassBird = 2,
    DesertBird = 3,
    SwampBird = 4,
    
    Goat = 5,
    MountainGoat = 6,
    
    DeerFemale = 7,
    DeerMale = 8,
    Elk = 9,
    
    BoarFemale = 10,
    BoarMale = 11,
    BoarElder = 12,
    
    PlainsOx = 13,
    TundraOx =14,
    
    JungleLargeBird = 15,
    DesertLargeBird = 16,

    // Monsters
    Jakyl = 17,
    AlphaJakyl = 18,
    KingJakyl = 19,
    
    RockCrab = 20,
    DesertCrab = 21,
    FrostCrab = 22,

    ForestToad = 23,
    SwampToad = 24,
    FrostToad = 25,

    Umbura = 26,
    AlphaUmbura = 27,
    KingUmbura = 28,

    Drone = 29,
    Soldier = 30,
    Queen = 31,

    Sentinel = 32,
    SentinelDungeonJakyl = 33,
    SentinelDungeonSkitch = 34,
    SentinelDungeonLargeJakyl = 35,

    CrabDungeonCrabBoss = 36,
    CrabDungeonCrabTrash = 37,

    SpiderDungeonEliteSpider = 38,
    SpiderDungeonSmallSpider = 39,
    SpiderDungeonSpiderNest = 40,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[repr(i32)]
pub enum EntityType {
    None = 0, // Probably not used
    Player = 1,
    Enemy = 2,
    Building = 3,
    Npc = 4,      // Probably not used
    Resource = 5, // Probably not used
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "NpcType")]
#[repr(i32)]
pub enum NpcType {
    None,
    Rumbagh,
    Svim,
    Heimlich,
    Twins,
    Brico,
    Tamer,
    Slayer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "CharacterStatType")]
#[repr(i32)]
pub enum CharacterStatType {
    MaxHealth,
    MaxStamina,
    PassiveHealthRegenRate,
    PassiveStaminaRegenRate,
    MovementMultiplier,
    SprintMultiplier,
    SprintStaminaDrain,
    Armor,
    CooldownMultiplier,
    HuntingWeaponPower,
    Strength,
    ColdProtection,
    HeatProtection,
    Evasion,
    ToolbeltSlots,
    CraftingSpeed,
    GatheringSpeed,
    BuildingSpeed,
    SatiationRegenRate,
    MaxSatiation,
    DefenseLevel,
    //DAB Note: values below are temporary, see comment inside `SkillType` definition
    //Profession stats
    ForestrySpeed,
    CarpentrySpeed,
    MasonrySpeed,
    MiningSpeed,
    SmithingSpeed,
    ScholarSpeed,
    LeatherworkingSpeed,
    HuntingSpeed,
    TailoringSpeed,
    FarmingSpeed,
    FishingSpeed,
    CookingSpeed,
    ForagingSpeed,
    ForestryPower,
    CarpentryPower,
    MasonryPower,
    MiningPower,
    SmithingPower,
    ScholarPower,
    LeatherworkingPower,
    HuntingPower,
    TailoringPower,
    FarmingPower,
    FishingPower,
    CookingPower,
    ForagingPower,
    //Move these values up once the temporary values get removed
    ActiveHealthRegenRate,
    ActiveStaminaRegenRate,
    ClimbProficiency,
    ExperienceRate,
    Accuracy,
    MaxTeleportationEnergy,
    TeleportationEnergyRegenRate,
    ConstructionPower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "ClaimType")]
pub enum ClaimType {
    Source,
    Extension,
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "BuildingSpawnType")]
pub enum BuildingSpawnType {
    TravelerCamp,
    Chest,
    Building,
    Resource,
    StationaryNpc,
    Paving,
    Enemy
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "ItemConversionLocationContext")]
pub enum ItemConversionLocationContext {
    None,
    Water,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "FootprintType")]
#[repr(i32)]
pub enum FootprintType {
    Hitbox,
    Walkable,
    Perimeter, //Only perimiter tiles can overlap
    WalkableResource, // everything can be built but there's a resource over which player can walk
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "InteriorSpawnType")]
#[repr(i32)]
pub enum InteriorSpawnType {
    Undefined,
    Traveler,
    Chest,
    Building,
    Resource,
    Enemy,
    Paving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[repr(i32)]
pub enum MovementType {
    None = 0b0000,
    Ground = 0b0001,
    Water = 0b0010,
    Amphibious = 0b0011,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
pub enum Rarity {
    Default,
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "SkillType")]
#[repr(i32)]
pub enum SkillType {
    //DAB Note: This is a temporary solution to avoid a significant rewrite of CharacterStats.
    //In the future, we can:
    //  - Replace all `XPower` and `XSpeed` entries in `CharacterStatType` with two `SkillPower` and `SkillType` entries, and
    //  - Change `CharacterStatsState` to have the following definition
    //      pub struct CharacterStatState {
    //          pub entity_id: u64,
    //          pub value: f32,
    //          pub skill_id: Option<i32>,
    //          pub stat_type: CharacterStatType,
    //      }
    //These changes will allow us to get rid of `SkillType` enum, and use a combination of `SkillDesc.skill_id` and `CharacterStatType` in its place.
    //This will also allow us to add new skills without code changes and would have a positive effect on `CharacterStats` performance
    None,
    ANY,
    Forestry,
    Carpentry,
    Masonry,
    Mining,
    Smithing,
    Scholar,
    Leatherworking,
    Hunting,
    Tailoring,
    Farming,
    Fishing,
    Cooking,
    Foraging,
    Construction,
    Exploration,
    Taming,
    Slayer,
    Trading,
    LoreKeeper,
    Sailing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "SkillCategory")]
pub enum SkillCategory {
    None,
    Profession,
    Adventure,
}

#[static_data_staging_table(skill_desc)]
#[derive(Clone, PartialEq, Debug)]
#[spacetimedb::table(name = skill_desc, public)]
pub struct SkillDesc {
    #[primary_key]
    pub id: i32,
    #[unique]
    pub skill_type: i32, //DAB Note: this is temporary, see comment inside `SkillType` definition

    pub name: String,
    pub description: String,
    pub icon_asset_name: String,
    pub title: String,
    pub skill_category: SkillCategory,
    pub max_level: i32,
}

#[static_data_staging_table(resource_desc)]
#[spacetimedb::table(name = resource_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub flattenable: bool,
    pub max_health: i32,
    pub ignore_damage: bool,
    pub despawn_time: f32,
    pub model_asset_name: String,
    pub icon_asset_name: String,
    pub on_destroy_yield: Vec<ItemStack>,
    pub on_destroy_yield_resource_id: i32,
    pub spawn_priority: i32,
    pub footprint: Vec<FootprintTile>,
    pub tier: i32,
    pub tag: String,
    pub rarity: Rarity,
    pub compendium_entry: bool,
    pub enemy_params_id: Vec<i32>,
    pub scheduled_respawn_time: f32,
    pub not_respawning: bool,
}

#[static_data_staging_table(cargo_desc)]
#[spacetimedb::table(name = cargo_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct CargoDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub volume: i32,
    pub secondary_knowledge_id: i32,
    pub model_asset_name: String,
    pub icon_asset_name: String,
    pub carried_model_asset_name: String,
    pub pick_up_animation_start: String,
    pub pick_up_animation_end: String,
    pub drop_animation_start: String,
    pub drop_animation_end: String,
    pub pick_up_time: f32,
    pub place_time: f32,
    pub animator_state: String,
    pub movement_modifier: f32,
    pub blocks_path: bool,
    pub on_destroy_yield_cargos: Vec<i32>,
    pub despawn_time: f32,
    pub tier: i32,
    pub tag: String,
    pub rarity: Rarity,
    pub not_pickupable: bool,
}

#[static_data_staging_table(pillar_shaping_desc)]
#[spacetimedb::table(name = pillar_shaping_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PillarShapingDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub input_cargo_id: i32,
    pub input_cargo_discovery_score: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub full_discovery_score: i32,
    pub duration: f32,
    pub prefab_address: String,
    pub tier: i32,
    pub icon_address: String,
    pub description: String,
}

#[static_data_staging_table(paving_tile_desc)]
#[spacetimedb::table(name = paving_tile_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PavingTileDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub input_cargo_id: i32,
    pub input_cargo_discovery_score: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub full_discovery_score: i32,
    pub paving_duration: f32,
    pub prefab_address: String,
    pub tier: i32,
    pub stat_effects: Vec<CsvStatEntry>,
    pub icon_address: String,
    pub description: String,
}

#[static_data_staging_table(building_type_desc)]
#[spacetimedb::table(name = building_type_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingTypeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub category: BuildingCategory,
    pub actions: Vec<String>,
}

#[static_data_staging_table(building_desc)]
#[spacetimedb::table(name = building_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingDesc {
    #[primary_key]
    pub id: i32,
    pub functions: Vec<BuildingFunction>,
    pub name: String,
    pub description: String,
    pub rested_buff_duration: i32,
    pub light_radius: i32,
    pub model_asset_name: String,
    pub icon_asset_name: String,
    pub unenterable: bool,
    pub wilderness: bool,
    pub footprint: Vec<FootprintTile>,
    pub max_health: i32,
    pub ignore_damage: bool,
    pub defense_level: i32,
    pub decay: f32,
    pub maintenance: f32,
    pub build_permission: BuildingInteractionLevel,
    pub interact_permission: BuildingInteractionLevel,
    pub has_action: bool,
    pub show_in_compendium: bool,
    pub is_ruins: bool,
    pub not_deconstructible: bool,
}

// A table that gets auto-built when static data is uploaded. Maps building function IDs to buildings that have that function.
// For example, there would be an entry for all smelters, an entry for all looms, and an entry for all kilns etc.
#[spacetimedb::table(name = building_function_type_mapping_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingFunctionTypeMappingDesc {
    #[primary_key]
    pub type_id: i32,
    pub desc_ids: Vec<i32>,
}

#[static_data_staging_table(item_desc)]
#[spacetimedb::table(name = item_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ItemDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub volume: i32,
    pub durability: i32,
    pub convert_to_on_durability_zero: i32,
    pub secondary_knowledge_id: i32,
    pub model_asset_name: String,
    pub icon_asset_name: String,
    pub tier: i32,
    pub tag: String,
    pub rarity: Rarity,
    pub compendium_entry: bool,
    pub item_list_id: i32,
}

#[static_data_staging_table(collectible_desc)]
#[spacetimedb::table(name = collectible_desc, public, index(name = item_deed_id, btree(columns = [item_deed_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct CollectibleDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub collectible_type: CollectibleType,
    pub invalidates_type: CollectibleType,
    pub auto_collect: bool,
    pub collectible_rarity: Rarity,
    pub starting_loadout: bool,
    pub locked: bool,
    pub variant: i32,
    pub color: String,
    pub emission: String,
    pub max_equip_count: i32,
    pub model_asset_name: String,
    pub variant_material: String,
    pub icon_asset_name: String,
    pub tag: String,
    pub display_string: String,
    pub item_deed_id: i32,
    pub required_knowledges_to_use: Vec<i32>, // Maps to Secondary Knowledge Ids
    pub required_knowledges_to_convert: Vec<i32>, // Maps to Secondary Knowledge Ids
}

#[static_data_staging_table(tool_type_desc)]
#[spacetimedb::table(name = tool_type_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ToolTypeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    #[unique]
    pub skill_id: i32,
}

#[static_data_staging_table(tool_desc)]
#[spacetimedb::table(name = tool_desc, public, index(name = item_id, btree(columns = [item_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ToolDesc {
    #[primary_key]
    pub id: i32,

    pub item_id: i32,
    pub tool_type: i32,
    pub level: i32,
    pub power: i32,
}

#[spacetimedb::table(name = deployable_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct DeployableDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    #[unique]
    pub deploy_from_collectible_id: i32,
    pub deploy_time: f32,
    pub deployable_type: DeployableType,
    pub pathfinding_id: i32,
    pub movement_type: MovementType,
    pub can_enter_portals: bool,
    pub speed: Vec<MovementSpeed>,
    pub use_player_speed_modifier: bool,
    pub placeable_on_land: bool,
    pub placeable_in_water: bool,
    pub capacity: i32,
    pub storage: i32,
    pub stockpile: i32,
    pub barter: i32, // max orders
    pub item_slot_size: i32,
    pub cargo_slot_size: i32,
    pub model_address: String,
    pub stats: Vec<CsvStatEntry>,
    pub player_animations_in_deployable_slots: Vec<i32>, // Converted to enum PlayerInDeployableState on the client
    pub allow_driver_extract: bool,
    pub allow_passenger_extract: bool,
    pub show_for_secs_after_owner_logout: i32, // -1 means we never hide the deployable
    pub allow_emote_while_driver: bool,
    pub allow_emote_while_passenger: bool,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub mounting_radius: f32,
}

#[spacetimedb::table(name = deployable_desc_v2, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct DeployableDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    #[unique]
    pub deploy_from_collectible_id: i32,
    pub deploy_time: f32,
    pub deployable_type: DeployableType,
    pub pathfinding_id: i32,
    pub movement_type: MovementType,
    pub can_enter_portals: bool,
    pub can_auto_follow: bool,
    pub affected_by_wind: f32,
    pub speed: Vec<MovementSpeed>,
    pub use_player_speed_modifier: bool,
    pub placeable_on_land: bool,
    pub placeable_in_water: bool, // obsolete, using max water depth instead
    pub capacity: i32,
    pub storage: i32,
    pub stockpile: i32,
    pub barter: i32, // max orders
    pub item_slot_size: i32,
    pub cargo_slot_size: i32,
    pub model_address: String,
    pub stats: Vec<CsvStatEntry>,
    pub player_animations_in_deployable_slots: Vec<i32>, // Converted to enum PlayerInDeployableState on the client
    pub allow_driver_extract: bool,
    pub allow_passenger_extract: bool,
    pub show_for_secs_after_owner_logout: i32, // -1 means we never hide the deployable
    pub allow_emote_while_driver: bool,
    pub allow_emote_while_passenger: bool,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub mounting_radius: f32,
}

#[spacetimedb::table(name = deployable_desc_v3, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct DeployableDescV3 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    #[unique]
    pub deploy_from_collectible_id: i32,
    pub deploy_time: f32,
    pub deployable_type: DeployableType,
    pub pathfinding_id: i32,
    pub movement_type: MovementType,
    pub can_enter_portals: bool,
    pub can_auto_follow: bool,
    pub affected_by_wind: f32,
    pub speed: Vec<MovementSpeed>,
    pub use_player_speed_modifier: bool,
    pub placeable_on_land: bool,
    pub placeable_in_water: bool,
    pub capacity: i32,
    pub storage: i32,
    pub stockpile: i32,
    pub barter: i32, // max orders
    pub item_slot_size: i32,
    pub cargo_slot_size: i32,
    pub model_address: String,
    pub stats: Vec<CsvStatEntry>,
    pub player_animations_in_deployable_slots: Vec<i32>, // Converted to enum PlayerInDeployableState on the client
    pub allow_driver_extract: bool,
    pub allow_passenger_extract: bool,
    pub show_for_secs_after_owner_logout: i32, // -1 means we never hide the deployable
    pub allow_emote_while_driver: bool,
    pub allow_emote_while_passenger: bool,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub mounting_radius: f32,
    pub radius: f32, // for now, range (in tiles) to halt pathfinding when extracting from a deployable. Using a f32 in case it's used later for some kind of pathfinding radius.
}

#[static_data_staging_table(deployable_desc)]
#[spacetimedb::table(name = deployable_desc_v4, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct DeployableDescV4 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    #[unique]
    pub deploy_from_collectible_id: i32,
    pub deploy_time: f32,
    pub deployable_type: DeployableType,
    pub pathfinding_id: i32,
    pub movement_type: MovementType,
    pub can_enter_portals: bool,
    pub can_auto_follow: bool,
    pub affected_by_wind: f32,
    pub speed: Vec<MovementSpeed>,
    pub use_player_speed_modifier: bool,
    pub placeable_on_land: bool,
    pub placeable_in_water: bool,
    pub capacity: i32,
    pub storage: i32,
    pub stockpile: i32,
    pub barter: i32, // max orders
    pub item_slot_size: i32,
    pub cargo_slot_size: i32,
    pub model_address: String,
    pub stats: Vec<CsvStatEntry>,
    pub player_animations_in_deployable_slots: Vec<i32>, // Converted to enum PlayerInDeployableState on the client
    pub allow_driver_extract: bool,
    pub allow_passenger_extract: bool,
    pub show_for_secs_after_owner_logout: i32, // -1 means we never hide the deployable
    pub allow_emote_while_driver: bool,
    pub allow_emote_while_passenger: bool,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub mounting_radius: f32,
    pub radius: f32, // for now, range (in tiles) to halt pathfinding when extracting from a deployable. Using a f32 in case it's used later for some kind of pathfinding radius.
    #[default(false)]
    pub allow_hunting: bool,
}

#[static_data_staging_table(crafting_recipe_desc)]
#[spacetimedb::table(name = crafting_recipe_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct CraftingRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub tool_durability_lost: i32,
    pub building_requirement: Option<BuildingRequirement>,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub discovery_triggers: Vec<i32>,
    pub required_claim_tech_id: i32,
    pub full_discovery_score: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub crafted_item_stacks: Vec<ItemStack>,
    pub actions_required: i32,
    pub tool_mesh_index: i32,
    pub recipe_performance_id: i32,
    pub required_knowledges: Vec<i32>,
    pub blocking_knowledges: Vec<i32>,
    pub hide_without_required_knowledge: bool,
    pub hide_with_blocking_knowledges: bool,
    pub allow_use_hands: bool,
    pub is_passive: bool,
}

#[spacetimedb::table(name = staged_construction_recipe_desc)]
#[spacetimedb::table(name = construction_recipe_desc, public, index(name = building_description_id, btree(columns = [building_description_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ConstructionRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub consumed_building: i32,
    pub required_interior_tier: i32,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub consumed_cargo_stacks: Vec<InputItemStack>,
    pub consumed_shards: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub required_claim_tech_id: i32,
    pub full_discovery_score: i32,
    pub tool_mesh_index: i32,
    pub building_description_id: i32,
    pub required_paving_tier: i32,
    pub actions_required: i32,
    pub instantly_built: bool,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(construction_recipe_desc_v2)]
#[spacetimedb::table(name = construction_recipe_desc_v2, public, index(name = building_description_id, btree(columns = [building_description_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ConstructionRecipeDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub consumed_building: i32,
    pub required_interior_tier: i32,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub consumed_cargo_stacks: Vec<InputItemStack>,
    pub consumed_shards: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub required_claim_tech_ids: Vec<i32>,
    pub full_discovery_score: i32,
    pub tool_mesh_index: i32,
    pub building_description_id: i32,
    pub required_paving_tier: i32,
    pub actions_required: i32,
    pub instantly_built: bool,
    pub recipe_performance_id: i32,
}

#[spacetimedb::table(name = staged_resource_placement_recipe_desc)]
#[spacetimedb::table(name = resource_placement_recipe_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ResourcePlacementRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub consumed_resource: i32,
    pub required_interior_tier: i32,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub consumed_cargo_stacks: Vec<InputItemStack>,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub required_claim_tech_id: i32,
    pub required_biomes: Vec<Biome>,
    pub full_discovery_score: i32,
    pub tool_mesh_index: i32,
    pub resource_description_id: i32,
    pub required_paving_tier: i32,
    pub actions_required: i32,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(resource_placement_recipe_desc_v2)]
#[spacetimedb::table(name = resource_placement_recipe_desc_v2, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ResourcePlacementRecipeDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub consumed_resource: i32,
    pub required_interior_tier: i32,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub consumed_cargo_stacks: Vec<InputItemStack>,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub required_claim_tech_ids: Vec<i32>,
    pub required_biomes: Vec<Biome>,
    pub full_discovery_score: i32,
    pub tool_mesh_index: i32,
    pub resource_description_id: i32,
    pub required_paving_tier: i32,
    pub actions_required: i32,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(resource_growth_recipe_desc)]
#[spacetimedb::table(name = resource_growth_recipe_desc, index(name = resource_id, btree(columns = [resource_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceGrowthRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub resource_id: i32,
    pub time: Vec<f32>,
    pub grown_resource_id: i32,
}

#[static_data_staging_table(extraction_recipe_desc)]
#[spacetimedb::table(name = extraction_recipe_desc, public, index(name = resource_id, btree(columns = [resource_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ExtractionRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub resource_id: i32,
    pub cargo_id: i32,
    pub discovery_triggers: Vec<i32>,
    pub required_knowledges: Vec<i32>,
    pub time_requirement: f32,
    pub stamina_requirement: f32,
    pub tool_durability_lost: i32,
    pub extracted_item_stacks: Vec<ProbabilisticItemStack>,
    pub consumed_item_stacks: Vec<InputItemStack>,
    pub range: i32,
    pub tool_requirements: Vec<ToolRequirement>,
    pub allow_use_hands: bool,
    pub level_requirements: Vec<LevelRequirement>,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub verb_phrase: String,
    pub tool_mesh_index: i32,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(deconstruction_recipe_desc)]
#[spacetimedb::table(name = deconstruction_recipe_desc, public, index(name = consumed_building, btree(columns = [consumed_building])))]
#[derive(Clone, PartialEq, Debug)]
pub struct DeconstructionRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub time_requirement: f32,
    pub consumed_building: i32,
    pub level_requirements: Vec<LevelRequirement>,
    pub tool_requirements: Vec<ToolRequirement>,
    pub output_item_stacks: Vec<ItemStack>,
    pub output_cargo_id: i32,
    pub experience_per_progress: Vec<ExperienceStackF32>,
    pub tool_mesh_index: i32,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(weapon_type_desc)]
#[spacetimedb::table(name = weapon_type_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct WeaponTypeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub hunting: bool,
}

#[static_data_staging_table(weapon_desc)]
#[spacetimedb::table(name = weapon_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct WeaponDesc {
    #[primary_key]
    pub item_id: i32,
    pub tier: i32,
    pub weapon_type: i32,
    pub min_damage: i32,
    pub max_damage: i32,
    pub cooldown: f32,
    pub stamina_use_multiplier: f32,
}


#[spacetimedb::table(name = parameters_desc, public)]
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ParametersDesc {
    #[primary_key]
    pub version: i32,

    pub default_speed: Vec<MovementSpeed>,
    pub default_num_inventory_pockets: i32,
    pub skill_yield_power_exponent: f32,
    pub skill_yield_log_base: f32,
    pub skill_yield_cutoff_percent: f32,
    pub tech_time_power_exponent: f32,
    pub tech_time_log_base: f32,
    pub min_seconds_to_passive_regen_health: i32,
    pub min_seconds_to_passive_regen_stamina: i32,
    pub repair_building_duration: i32,
    pub repair_building_max_repair_percent: f32,
    pub repair_building_show_damage_percent: f32,
    pub environment_debuff_tick_millis: i32,
    pub enemy_spawn_agent_wait_time: i32,
    pub discovery_range: i32,
    pub max_build_range: i32,
    pub deconstruct_default_time: f32,
    pub respawn_seconds: i32,
    pub player_climb_height: f32,
    pub player_jump_height: f32,
    pub player_swim_height: f32,
    pub daytime: i32,
    pub nighttime: i32,
    pub aggro_range: i32,
    pub max_cargo_per_hex: i32,
    pub resources_regen_loops: i32,
    pub selected_traveler_order_count: i32,
    pub resources_regen_tick_millis: i32,
    pub building_decay_tick_millis: i32,
    pub max_traded_items: i32,
    pub max_trade_distance_large_tiles: i32,
    pub min_distance_between_claims: i32,
    pub combat_manager_tick_millis: i32,
    pub disengage_range: i32,
    pub disengage_time_millis: i32,
    pub action_slots: i32,
    pub starting_supplies: i32,
    pub show_shield_bar_percent: f32,
    pub swim_sprint_speed_multiplier: f32,
    pub loot_chest_despawn_time_seconds: f32,
    pub deployable_disembark_max_elevation: i32,
    pub default_num_toolbelt_pockets: i32,
    pub resource_growth_tick_rate_milliseconds: i32,
    pub rent_deposit_days: i32,
    pub rent_collection_time_of_day: f32,
    pub rent_eviction_compensation: f32,
    pub max_rental_deposit_days: i32,
    pub recommended_achievements: i32,
    pub empire_decay_tick_millis: i32,
    pub empire_siege_tick_millis: i32,
    pub empire_siege_raise_pct: f32,
    pub empire_default_nobility_threshold: i32,
    pub empire_shard_cost: i32,
    pub empire_starting_shards: i32,
    pub empire_node_max_energy: i32,
    pub empire_node_starting_energy: i32,
    pub empire_min_siege_distance: i32,
    pub empire_max_siege_distance: i32,
    pub daily_shards: i32,
    pub hexite_capsule_supply_cost: i32,
    pub hexite_capsule_shard_cost: i32,
    pub hexite_capsule_craft_time_seconds: i32,
    pub crafting_lock_duration_secs: i32,
    pub num_toolbars: i32,
    pub starving_tick_millis: i32,
    pub starving_damage: f32,
    pub claim_stability_param_m: f32,
    pub claim_stability_param_b: f32,
    pub player_regen_tick_millis: i32,
    pub enemy_regen_tick_millis: i32,
    pub teleportation_energy_regen_tick_millis: i32,
    pub auto_respawn_attempts: i32,
    pub player_pathfinding_id: i32,
    pub nearby_flame_buff_id: i32,
    pub floating_origin_distance_threshold: i32,
    pub withdraw_from_deployables_range: i32,
    pub deposit_to_deployables_range: i32,
    pub sign_in_aggro_immunity: i32,
    pub respawn_aggro_immunity: i32,
    pub new_user_aggro_immunity: i32,
    pub terraform_experience_per_progress: f32,
    pub dropped_inventory_ownership_seconds: i32,
    pub dropped_inventory_despawn_seconds: i32,
    pub enemy_loot_ownership_seconds: i32,
    pub traveler_tasks_per_npc: i32,
    pub traveler_tasks_times_of_day: Vec<i32>, // 0h-23h format, UTC
    pub teleport_channel_time_home: f32,
    pub teleport_channel_time_waystone: f32,
    pub teleportation_home_energy_cost: f32,
    pub teleportation_base_energy_cost: f32,
    pub teleportation_cost_per_large_tile: f32,
    pub teleportation_full_inventory_multiplier: f32,
    pub region_crossover_distance_large_tiles: i32,
    pub item_recovery_range: i32,
    pub quick_board_range: f32,
    pub duel_range: f32,
    pub duel_out_of_range_grace_period_millis: i32,
    pub player_housing_same_region_move_time_minutes: i32,  // [MIGRATION] DEPRECATED
    pub player_housing_eviction_time_minutes: i32,
    pub player_housing_income_time_of_day: f32,
}

#[static_data_staging_table(parameters_desc)]
#[spacetimedb::table(name = parameters_desc_v2, public)]
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ParametersDescV2 {
    #[primary_key]
    pub version: i32,

    pub default_speed: Vec<MovementSpeed>,
    pub default_num_inventory_pockets: i32,
    pub skill_yield_power_exponent: f32,
    pub skill_yield_log_base: f32,
    pub skill_yield_cutoff_percent: f32,
    pub tech_time_power_exponent: f32,
    pub tech_time_log_base: f32,
    pub min_seconds_to_passive_regen_health: i32,
    pub min_seconds_to_passive_regen_stamina: i32,
    pub repair_building_duration: i32,
    pub repair_building_max_repair_percent: f32,
    pub repair_building_show_damage_percent: f32,
    pub environment_debuff_tick_millis: i32,
    pub enemy_spawn_agent_wait_time: i32,
    pub discovery_range: i32,
    pub max_build_range: i32,
    pub deconstruct_default_time: f32,
    pub respawn_seconds: i32,
    pub player_climb_height: f32,
    pub player_jump_height: f32,
    pub player_swim_height: f32,
    pub daytime: i32,
    pub nighttime: i32,
    pub aggro_range: i32,
    pub max_cargo_per_hex: i32,
    pub resources_regen_loops: i32,
    pub selected_traveler_order_count: i32,
    pub resources_regen_tick_millis: i32,
    pub building_decay_tick_millis: i32,
    pub max_traded_items: i32,
    pub max_trade_distance_large_tiles: i32,
    pub min_distance_between_claims: i32,
    pub combat_manager_tick_millis: i32,
    pub disengage_range: i32,
    pub disengage_time_millis: i32,
    pub action_slots: i32,
    pub starting_supplies: i32,
    pub show_shield_bar_percent: f32,
    pub swim_sprint_speed_multiplier: f32,
    pub loot_chest_despawn_time_seconds: f32,
    pub deployable_disembark_max_elevation: i32,
    pub default_num_toolbelt_pockets: i32,
    pub resource_growth_tick_rate_milliseconds: i32,
    pub rent_deposit_days: i32,
    pub rent_collection_time_of_day: f32,
    pub rent_eviction_compensation: f32,
    pub max_rental_deposit_days: i32,
    pub recommended_achievements: i32,
    pub empire_decay_tick_millis: i32,
    pub empire_siege_tick_millis: i32,
    pub empire_siege_raise_pct: f32,
    pub empire_default_nobility_threshold: i32,
    pub empire_shard_cost: i32,
    pub empire_starting_shards: i32,
    pub empire_node_max_energy: i32,
    pub empire_node_starting_energy: i32,
    pub empire_min_siege_distance: i32,
    pub empire_max_siege_distance: i32,
    pub daily_shards: i32,
    pub hexite_capsule_supply_cost: i32,
    pub hexite_capsule_shard_cost: i32,
    pub hexite_capsule_craft_time_seconds: i32,
    pub crafting_lock_duration_secs: i32,
    pub num_toolbars: i32,
    pub starving_tick_millis: i32,
    pub starving_damage: f32,
    pub claim_stability_param_m: f32,
    pub claim_stability_param_b: f32,
    pub player_regen_tick_millis: i32,
    pub enemy_regen_tick_millis: i32,
    pub teleportation_energy_regen_tick_millis: i32,
    pub auto_respawn_attempts: i32,
    pub player_pathfinding_id: i32,
    pub nearby_flame_buff_id: i32,
    pub floating_origin_distance_threshold: i32,
    pub withdraw_from_deployables_range: i32,
    pub deposit_to_deployables_range: i32,
    pub sign_in_aggro_immunity: i32,
    pub respawn_aggro_immunity: i32,
    pub new_user_aggro_immunity: i32,
    pub terraform_experience_per_progress: f32,
    pub dropped_inventory_ownership_seconds: i32,
    pub dropped_inventory_despawn_seconds: i32,
    pub enemy_loot_ownership_seconds: i32,
    pub traveler_tasks_per_npc: i32,
    pub traveler_tasks_times_of_day: Vec<i32>, // 0h-23h format, UTC
    pub teleport_channel_time_home: f32,
    pub teleport_channel_time_waystone: f32,
    pub teleportation_home_energy_cost: f32,
    pub teleportation_base_energy_cost: f32,
    pub teleportation_cost_per_large_tile: f32,
    pub teleportation_full_inventory_multiplier: f32,
    pub region_crossover_distance_large_tiles: i32,
    pub item_recovery_range: i32,
    pub quick_board_range: f32,
    pub duel_range: f32,
    pub duel_out_of_range_grace_period_millis: i32,
    pub player_housing_same_region_move_time_minutes: i32,  // [MIGRATION] DEPRECATED
    pub player_housing_eviction_time_minutes: i32,
    pub player_housing_income_time_of_day: f32,
    pub co_owner_take_ownership_supply_time: i32,
    pub officer_take_ownership_supply_time: i32,
    pub member_take_ownership_supply_time: i32,
}

#[spacetimedb::table(name = parameters_player_move_desc)]
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ParametersPlayerMoveDesc {
    #[primary_key]
    pub version: i32,
    pub default_speed: Vec<MovementSpeed>,
}

#[static_data_staging_table(private_parameters_desc)]
#[spacetimedb::table(name = private_parameters_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct PrivateParametersDesc {
    #[primary_key]
    pub version: i32,

    pub move_validation: MoveValidationParamsDesc,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct MoveValidationParamsDesc {
    pub strike_count_before_move_validation_failure: i32,
    pub strike_counter_time_window_sec: i32,
}

#[static_data_staging_table(clothing_desc)]
#[spacetimedb::table(name = clothing_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ClothingDesc {
    #[primary_key]
    pub item_id: i32,
    pub mask: ClothingMask,
}

#[static_data_staging_table(knowledge_scroll_desc)]
#[spacetimedb::table(name = knowledge_scroll_desc, public, index(name = known_by_default, btree(columns = [known_by_default])))]
#[derive(Clone, PartialEq, Debug)]
pub struct KnowledgeScrollDesc {
    #[primary_key]
    pub item_id: i32,
    pub scroll_type: i32,
    pub secondary_knowledge_id: i32,
    pub known_by_default: bool,
    pub auto_collect: bool,
    pub title: String,
    pub tag: String,
    pub content: String,
}

#[static_data_staging_table(knowledge_scroll_type_desc)]
#[spacetimedb::table(name = knowledge_scroll_type_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct KnowledgeScrollTypeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
}

#[static_data_staging_table(equipment_desc)]
#[spacetimedb::table(name = equipment_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EquipmentDesc {
    #[primary_key]
    pub item_id: i32,
    pub slots: Vec<EquipmentSlotType>,
    pub visual_type: EquipmentVisualType,
    pub level_requirement: Option<LevelRequirement>,
    pub clothing_visual: Option<ClothingVisual>,
    pub hand_equipment_visual: Option<HandEquipmentVisual>,
    pub stats: Vec<CsvStatEntry>,
    pub required_achievements: Vec<i32>, // Maps to AchievementDesc.id
    pub required_knowledges: Vec<i32>, // Maps to Secondary Knowledge Ids
}

#[static_data_staging_table(buff_type_desc)]
#[spacetimedb::table(name = buff_type_desc, public, index(name = category, btree(columns = [category])))]
#[derive(Clone, PartialEq, Debug)]
pub struct BuffTypeDesc {
    #[primary_key]
    pub id: i32,

    pub name: String,
    pub category: i32,
}

#[derive(SpacetimeType)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuffEffect {
    pub buff_id: i32,
    pub duration: Option<i32>,
}

#[static_data_staging_table(buff_desc)]
#[spacetimedb::table(name = buff_desc, public, index(name = buff_type_id, btree(columns = [buff_type_id])))]
// Index on `buff_type` so that `ActiveBuffState::active_buff_of_type`
// can do efficient lookups.
#[derive(Clone, PartialEq, Debug)]
pub struct BuffDesc {
    #[primary_key]
    pub id: i32,

    pub buff_type_id: i32,
    pub description: String,
    pub priority: i32,
    pub duration: i32,
    pub warn_time: f32,
    pub online_timestamp: bool,
    pub beneficial: bool,
    pub icon_asset_name: String,
    pub stats: Vec<CsvStatEntry>,
    pub vfx: String,
    pub vfx_attachment_point: VfxAttachmentPoint,
}

#[static_data_staging_table(teleport_item_desc)]
#[spacetimedb::table(name = teleport_item_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct TeleportItemDesc {
    #[primary_key]
    pub id: i32,

    pub name: String,
    pub buff_id: i32,
}

#[static_data_staging_table(environment_debuff_desc)]
#[spacetimedb::table(name = environment_debuff_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EnvironmentDebuffDesc {
    #[primary_key]
    pub buff_id: i32,
    pub resistance_type: EnvironmentResistanceType,
    pub resistance_level: i32,
    pub ground_damage: i32,
    pub water_damage: i32,
    pub resistance_unmet_text: String,
    pub resistance_met_text: String,
}

#[static_data_staging_table(food_desc)]
#[spacetimedb::table(name = food_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct FoodDesc {
    #[primary_key]
    pub item_id: i32,
    pub hp: f32,
    pub up_to_hp: f32,
    pub stamina: f32,
    pub up_to_stamina: f32,
    pub hunger: f32,
    pub teleportation_energy: f32,
    pub consumable_while_in_combat: bool,
    pub buffs: Vec<BuffEffect>,
}

#[static_data_staging_table(enemy_desc)]
#[spacetimedb::table(name = enemy_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EnemyDesc {
    #[primary_key]
    pub enemy_type: i32,
    pub name: String,
    pub pathfinding_id: i32,
    pub targeting_matrix_id: i32,
    pub combat_actions_ids: Vec<i32>,
    pub description: String,
    pub min_speed: i32,
    pub max_speed: i32,
    pub radius: f32,
    pub awareness_destination_threshold: f32,
    pub min_awareness_tick_sec: f32,
    pub max_awareness_tick_sec: f32,
    pub max_health: i32,
    pub ignore_damage: bool,
    pub health_regen_quantity: f32,
    pub armor: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub strength: i32,  // affected by combat action speed
    pub min_damage: i32,
    pub max_damage: i32,
    pub cooldown_multiplier: f32,
    pub daytime_detect_range: i32,
    pub daytime_aggro_range: i32,
    pub daytime_deaggro_range: i32,
    pub nighttime_detect_range: i32,
    pub nighttime_aggro_range: i32,
    pub nighttime_deaggro_range: i32,
    pub evade_range: i32,
    pub deaggro_health_threshold: f32,
    pub attack_level: i32,
    pub defense_level: i32,
    pub prefab_address: String,
    pub icon_address: String,
    pub extracted_item_stacks: Vec<ProbabilisticItemStack>,
    pub experience_per_damage_dealt: Vec<ExperienceStackF32>,
    pub tier: i32,
    pub tag: String,
    pub rarity: Rarity,
    pub huntable: bool,
}

#[static_data_staging_table(enemy_scaling_desc)]
#[spacetimedb::table(name = enemy_scaling_desc, public, index(name = enemy_type_id, btree(columns = [enemy_type_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct EnemyScalingDesc {
    #[primary_key]
    pub id: i32,
    pub enemy_type_id: i32,
    pub required_players_count: i32,
    pub scaled_armor_bonus: i32,
    pub strength_bonus: i32,
    pub accuracy_bonus: i32,
    pub evasion_bonus: i32,
    pub min_damage_bonus: i32,
    pub max_damage_bonus: i32,
}

#[spacetimedb::table(name = contribution_loot_desc, public, index(name = enemy_type_id, btree(columns = [enemy_type_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ContributionLootDesc {
    #[primary_key]
    pub id: i32,
    pub enemy_type_id: i32,
    pub item_list_id: i32,
    pub minimum_contribution: i32,
}


#[static_data_staging_table(contribution_loot_desc)]
#[spacetimedb::table(name = contribution_loot_desc_v2, public, index(name = enemy_type_id, btree(columns = [enemy_type_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ContributionLootDescV2 {
    #[primary_key]
    pub id: i32,
    pub enemy_type_id: i32,
    pub item_list_id: i32,
    pub minimum_contribution: i32,
    pub weighted: bool,
}

#[static_data_staging_table(npc_desc)]
#[spacetimedb::table(name = npc_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct NpcDesc {
    #[primary_key]
    pub npc_type: i32,
    pub name: String,
    pub population: f32,
    pub speed: i32,
    pub min_time_at_ruin: i32,
    pub max_time_at_ruin: i32,
    pub prefab_address: String,
    pub icon_address: String,
    pub force_market_mode: bool,
    pub task_skill_check: Vec<i32>   
}

#[static_data_staging_table(enemy_ai_params_desc)]
#[spacetimedb::table(name = enemy_ai_params_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EnemyAiParamsDesc {
    #[primary_key]
    pub id: i32,
    pub enemy_type: EnemyType,
    pub biome: Biome,
    pub avg_herd_size: i32,
    pub var_herd_size: f32,
    pub herds_per_chunk: f32,
    pub roaming_distance: i32,
    pub spawn_eagerness: f32,
    pub time_of_day_start: f32,
    pub time_of_day_end: f32,
    pub spawn_frequency_minutes: f32,
}


// [MIGRATION WORK-AROUND] This is to go around the fact that we can't use migration yet. These should be additional fields to the CombatActionDescV3 rows.
#[static_data_staging_table(combat_action_multi_hit_desc)]
#[spacetimedb::table(name = combat_action_multi_hit_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct CombatActionMultiHitDesc {
    #[primary_key]
    pub id: i32,
    pub area_footprint: Vec<FootprintTile>,  // unused for now
    pub max_secondary_targets: i32,
    pub secondary_target_multiplier: f32,
}


#[spacetimedb::table(name = combat_action_desc, public, index(name = learned_by_player, btree(columns = [learned_by_player])))]
#[derive(Clone, PartialEq, Debug)]
pub struct CombatActionDesc {
    pub name: String,
    #[primary_key]
    pub id: i32,
    pub learned_by_player: bool,
    pub range: u32,
    pub max_range: u32,
    pub auto_cast: bool,
    pub weapon_type_requirements: Vec<i32>,
    pub lead_in_time: f32,
    pub can_move_during_lead_in: bool,
    pub cooldown: f32,
    pub global_cooldown: f32,
    pub ignore_global_cooldown: bool,
    pub strength_multiplier: f32,
    pub accuracy_multiplier: f32,
    pub stamina_use: f32,
    pub weapon_durability_lost: i32,
    pub self_buffs: Vec<BuffEffect>,
    pub target_buffs: Vec<BuffEffect>,
    pub level_requirement: Option<LevelRequirement>,
    pub icon_asset_name: String,
    pub player_animation_id: i32,
    pub npc_animation_name: String,
    pub hit_vfx: String,
    pub projectile_speed: f32,
    pub projectile_vfx: String,
    pub description: String,
    pub self_threat_against_buildings: f32,
    pub self_threat_against_enemies: f32,
    pub base_threat: f32,
    pub threat_per_damage: f32,
}

#[spacetimedb::table(name = combat_action_desc_v2, public, index(name = learned_by_player, btree(columns = [learned_by_player])))]
#[derive(Clone, PartialEq, Debug)]
pub struct CombatActionDescV2 {
    pub name: String,
    #[primary_key]
    pub id: i32,
    pub learned_by_player: bool,
    pub range: u32,
    pub max_range: f32,
    pub auto_cast: bool,
    pub weapon_type_requirements: Vec<i32>,
    pub lead_in_time: f32,
    pub can_move_during_lead_in: bool,
    pub cooldown: f32,
    pub global_cooldown: f32,
    pub ignore_global_cooldown: bool,
    pub strength_multiplier: f32,
    pub accuracy_multiplier: f32,
    pub stamina_use: f32,
    pub weapon_durability_lost: i32,
    pub self_buffs: Vec<BuffEffect>,
    pub target_buffs: Vec<BuffEffect>,
    pub level_requirement: Option<LevelRequirement>,
    pub icon_asset_name: String,
    pub player_animation_id: i32,
    pub npc_animation_name: String,
    pub hit_vfx: String,
    pub projectile_speed: f32,
    pub projectile_vfx: String,
    pub description: String,
    pub self_threat_against_buildings: f32,
    pub self_threat_against_enemies: f32,
    pub base_threat: f32,
    pub threat_per_damage: f32,
    pub is_self_targeting: bool,
    pub is_taunt_action: bool,
}

#[static_data_staging_table(combat_action_desc_v3)]
#[spacetimedb::table(name = combat_action_desc_v3, public, index(name = learned_by_player, btree(columns = [learned_by_player])))]
#[derive(Clone, PartialEq, Debug)]
pub struct CombatActionDescV3 {
    pub name: String,
    #[primary_key]
    pub id: i32,
    pub learned_by_player: bool,
    pub range: u32,
    pub max_range: f32,
    pub auto_cast: bool,
    pub weapon_type_requirements: Vec<i32>,
    pub lead_in_time: f32,
    pub inaction_time: f32,
    pub can_move_during_lead_in: bool,
    pub cooldown: f32,
    pub global_cooldown: f32,
    pub ignore_global_cooldown: bool,
    pub strength_multiplier: f32,
    pub accuracy_multiplier: f32,
    pub stamina_use: f32,
    pub weapon_durability_lost: i32,
    pub self_buffs: Vec<BuffEffect>,
    pub target_buffs: Vec<BuffEffect>,
    pub level_requirement: Option<LevelRequirement>,
    pub icon_asset_name: String,
    pub player_animation_id: i32,
    pub npc_animation_name: String,
    pub hit_vfx: String,
    pub projectile_speed: f32,
    pub projectile_vfx: String,
    pub description: String,
    pub self_threat_against_buildings: f32,
    pub self_threat_against_enemies: f32,
    pub base_threat: f32,
    pub threat_per_damage: f32,
    pub is_self_targeting: bool,
    pub is_taunt_action: bool,
}


#[static_data_staging_table(traveler_trade_order_desc)]
#[spacetimedb::table(name = traveler_trade_order_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct TravelerTradeOrderDesc {
    #[primary_key]
    pub id: i32,
    pub starting_stock: i32,
    pub always_offered: bool,
    pub traveler: NpcType,
    pub offer_items: Vec<ItemStack>,
    pub offer_cargo_id: Vec<i32>,           // # MIGRATION # OBSOLETE - just keep offer_items for both items and cargo
    pub required_items: Vec<ItemStack>,
    pub required_cargo_id: Vec<i32>,        // # MIGRATION # OBSOLETE - just keep required_items for both items and cargo
    pub level_requirements: Vec<LevelRequirement>,
    pub achievement_requirements: Vec<i32>,
    pub hide_if_requirements_are_not_met: bool,
    pub required_knowledges: Vec<i32>,
    pub hide_without_required_knowledge: bool,
    pub blocking_knowledges: Vec<i32>,
    pub hide_with_blocking_knowledges: bool,
}

#[static_data_staging_table(traveler_task_desc)]
#[spacetimedb::table(name = traveler_task_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct TravelerTaskDesc {
    #[primary_key]
    pub id: i32,
    pub level_requirement: CappedLevelRequirement,
    pub required_items: Vec<ItemStack>,
    pub rewarded_items: Vec<ItemStack>,
    pub rewarded_experience: ExperienceStackF32,
    pub description: String,
}


#[static_data_staging_table(character_stat_desc)]
#[spacetimedb::table(name = character_stat_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct CharacterStatDesc {
    #[primary_key]
    pub stat_type: i32,
    pub name: String,
    pub value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub suffix: String,
    pub desc: String,
}

#[static_data_staging_table(building_repairs_desc)]
#[spacetimedb::table(name = building_repairs_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingRepairsDesc {
    #[primary_key]
    pub cargo_id: i32,
    pub repair_value: i32,
}

#[static_data_staging_table(building_claim_desc)]
#[spacetimedb::table(name = building_claim_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingClaimDesc {
    #[primary_key]
    pub building_id: i32,
    pub claim_type: ClaimType,
    pub radius: i32,
    pub tier: i32,
}

#[static_data_staging_table(claim_tile_cost)]
#[spacetimedb::table(name = claim_tile_cost, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimTileCost {
    #[primary_key]
    pub tile_count: i32,
    pub cost_per_tile: f32,
}

#[static_data_staging_table(terraform_recipe_desc)]
#[spacetimedb::table(name = terraform_recipe_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct TerraformRecipeDesc {
    #[primary_key]
    pub difference: i16,
    pub actions_count: i32,
    pub tool_requirement: Option<ToolRequirement>,
    pub stamina_per_action: f32,
    pub time_per_action: f32,
    pub tool_mesh_index: i32,
    pub recipe_performance_id: i32,
}

#[spacetimedb::table(name = emote_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmoteDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub animation: String,
    pub duration: f32,
    pub key_code: String,
    pub command_line: String,
    pub allow_while_mounted: bool,
    pub allow_while_moving: bool,
}

#[static_data_staging_table(emote_desc)]
#[spacetimedb::table(name = emote_desc_v2, public, 
    index(name = enabled_by_collectible_id, btree(columns = [enabled_by_collectible_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct EmoteDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub animation: String,
    pub duration: f32,
    pub key_code: String,
    pub command_line: String,
    pub allow_while_mounted: bool,
    pub allow_while_moving: bool,
    #[default(0)]
    pub enabled_by_collectible_id: i32,
    #[default(0)]
    pub tool_type: i32,
    #[default(0)]
    pub tool_mesh_index: i32,
}

#[static_data_staging_table(empire_notification_desc)]
#[spacetimedb::table(name = empire_notification_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireNotificationDesc {
    #[primary_key]
    pub id: i32,
    pub notification_type: EmpireNotificationType,
    pub priority: i32,
    pub show_on_login: bool,
    pub text: String,
}

#[static_data_staging_table(empire_territory_desc)]
#[spacetimedb::table(name = empire_territory_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireTerritoryDesc {
    #[auto_inc]
    #[primary_key]
    pub id: u8,
    pub chunks: u16,
    pub ranks: Vec<u8>,
    pub crown_collectible_id: i32,
}

#[static_data_staging_table(empire_supplies_desc)]
#[spacetimedb::table(name = empire_supplies_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireSuppliesDesc {
    #[primary_key]
    pub cargo_id: i32,
    pub energy: i32,
}

#[static_data_staging_table(empire_rank_desc)]
#[spacetimedb::table(name = empire_rank_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireRankDesc {
    #[primary_key]
    pub rank: i32,
    pub title: String,
    pub max_count: Option<i32>,
    pub permissions: Vec<bool>, // matches EmpirePermission enum
}

#[static_data_staging_table(targeting_matrix_desc)]
#[spacetimedb::table(name = targeting_matrix_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct TargetingMatrixDesc {
    #[primary_key]
    pub id: i32,
    pub interact: bool,
    pub categories_attacked: Vec<i32>,
    pub categories_weights: Vec<f32>,
}

#[static_data_staging_table(loot_table_desc)]
#[spacetimedb::table(name = loot_table_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct LootTableDesc {
    #[primary_key]
    pub id: i32,
    pub loot_rarity: i32,
    pub loot_item_stacks: Vec<ProbabilisticItemStack>,
}

#[static_data_staging_table(loot_rarity_desc)]
#[spacetimedb::table(name = loot_rarity_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct LootRarityDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
}

#[static_data_staging_table(loot_chest_desc)]
#[spacetimedb::table(name = loot_chest_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct LootChestDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub chest_rarity: i32,
    pub prefab_address: String,
    pub loot_tables: Vec<i32>,
}

#[static_data_staging_table(building_spawn_desc)]
#[spacetimedb::table(name = building_spawn_desc, index(name = building_id, btree(columns = [building_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingSpawnDesc {
    #[primary_key]
    pub id: i32,
    pub building_id: i32,
    pub x: i32,
    pub z: i32,
    pub direction: i32,
    pub spawn_type: BuildingSpawnType,
    pub traveler_type: Option<NpcType>,
    pub enemy_type: Option<EnemyType>,
    pub spawn_ids: Vec<i32>,
    pub respawn_time_min: f32,
    pub respawn_time_max: f32,
}

#[static_data_staging_table(resource_clump_desc)]
#[spacetimedb::table(name = resource_clump_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceClumpDesc {
    #[primary_key]
    pub id: i32,
    pub resource_id: Vec<i32>,
    pub x: Vec<i32>,
    pub z: Vec<i32>,
    pub direction: Vec<i32>,
}

#[spacetimedb::table(name = single_resource_to_clump_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct SingleResourceToClumpDesc {
    // DAB Note: technically this isn't unique, as different clumps can have the same single resource, but effectively since they're always centered there's no point for duplicates
    // so in the meantime, we only keep tab of the first clump referencing a single instance of this resource for spawning.
    #[primary_key]
    pub resource_id: i32,
    #[unique]
    pub clump_id: i32,
}

#[static_data_staging_table(chest_rarity_desc)]
#[spacetimedb::table(name = chest_rarity_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct ChestRarityDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub loot_rarities: Vec<ChestLootRarity>,
}

#[static_data_staging_table(secondary_knowledge_desc)]
#[spacetimedb::table(name = secondary_knowledge_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct SecondaryKnowledgeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
}

#[static_data_staging_table(item_conversion_recipe_desc)]
#[spacetimedb::table(name = item_conversion_recipe_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ItemConversionRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub time_cost: u32,
    pub stamina_cost: u32,
    pub location_context: i32,
    pub string_context: String,
    pub output_item: Option<ItemStack>,
    pub input_items: Vec<ItemStack>,
    pub required_equipment_id: i32,
    pub required_equipment_tier: i32,
    pub allow_use_hands: bool,
    pub recipe_performance_id: i32,
}

#[static_data_staging_table(interior_shape_desc)]
#[spacetimedb::table(name = interior_shape_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorShapeDesc {
    #[primary_key]
    pub id: i32,

    pub footprint: Vec<FootprintTile>,
    pub min_x: i32,
    pub min_z: i32,
    pub size_x: i32,
    pub size_z: i32,
}

#[static_data_staging_table(interior_instance_desc)]
#[spacetimedb::table(name = interior_instance_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorInstanceDesc {
    #[primary_key]
    pub id: i32,

    pub interior_shape_id: i32,
    pub tier: i32,
    pub biome: u8,
    pub rentable: bool,
    pub generate_walls_mesh: bool,
    pub generate_floor_mesh: bool,
    pub default_lighting: bool,
    pub interior_model: String,
    pub wall_material: String,
    pub floor_material: String,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub min_angle: f32,
    pub max_angle: f32,
    pub intertior_environment_id: Option<i32>,
}

#[static_data_staging_table(interior_environment_desc)]
#[spacetimedb::table(name = interior_environment_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorEnvironmentDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,

    pub skybox_material: String,
    pub realtime_shadow_color: u32,

    pub lighting_source: u8,
    pub lighting_skybox_intensity: f32,
    pub lighting_gradient_sky_color: u32,
    pub lighting_gradient_equator_color: u32,
    pub lighting_gradient_ground_color: u32,
    pub lighting_color_ambient_color: u32,

    pub fog_enable: bool,
    pub fog_color: u32,
    pub fog_mode: u8,
    pub fog_density: f32,
    pub fog_start_distance: f32,
    pub fog_end_distance: f32,
}

#[static_data_staging_table(interior_network_desc)]
#[spacetimedb::table(name = interior_network_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorNetworkDesc {
    #[primary_key]
    pub building_id: i32,

    pub dimension_type: DimensionType,
    pub trigger_collapse_time: u32,
    pub respawn_time: u32,
    pub child_interior_instances: Vec<i32>, //Cached during validation
}

#[spacetimedb::table(name = building_portal_desc, public, index(name = building_id, btree(columns = [building_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingPortalDesc { // DEPRECATED

    #[primary_key]
    pub id: i32,

    pub name: String,
    pub building_id: i32,
    pub allow_deployables: bool,
    pub pos_x: i32,
    pub pos_z: i32,
}

#[static_data_staging_table(building_portal_desc_v2)]
#[spacetimedb::table(name = building_portal_desc_v2, public, index(name = building_id, btree(columns = [building_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct BuildingPortalDescV2 {
    #[primary_key]
    pub id: i32,

    pub name: String,
    pub building_id: i32,
    pub allow_deployables: bool,
    pub enemy_lock: bool,
    pub pos_x: i32,
    pub pos_z: i32,
}

#[static_data_staging_table(interior_spawn_desc)]
#[spacetimedb::table(name = interior_spawn_desc, public, index(name = interior_instance_id, btree(columns = [interior_instance_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorSpawnDesc {
    #[primary_key]
    pub id: i32,

    pub name: String,
    pub interior_instance_id: i32,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub direction: i32,
    pub spawn_type: InteriorSpawnType,
    pub building_id: i32,
    pub paving_id: i32,
    pub loot_chests: Vec<i32>,
    pub resource_clump_id: i32,
    pub enemy_type: EnemyType,  // [MIGRATION TODO] OBSOLETE - This should be enemy_ai_desc_id but we're using [traveler_ruin_entity_id] instead for now
    pub traveler_type: NpcType,
    pub traveler_ruin_entity_id: i32,       // [MIGRATION TODO] This is EITHER a Traveler Id or an EnemyAIDesc Id - This is the most horrible hack ever, but the enemy type now spawns EnemyAiParamsDesc and we need a i32 for that 
    pub respawn: bool,
    pub collapse_trigger: bool,
}

#[static_data_staging_table(interior_portal_connections_desc)]
#[spacetimedb::table(name = interior_portal_connections_desc, public, 
    index(name = entrance_portal_id, btree(columns = [entrance_portal_id])),
    index(name = entrance_spawn_id, btree(columns = [entrance_spawn_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct InteriorPortalConnectionsDesc {
    #[primary_key]
    pub id: i32,

    pub entrance_spawn_id: i32,
    pub entrance_portal_id: i32,
    pub exit_spawn_id: i32,
    pub exit_portal_id: i32,
}

#[static_data_staging_table(item_list_desc)]
#[spacetimedb::table(name = item_list_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ItemListDesc {
    #[primary_key]
    pub id: i32,

    pub name: String,
    pub possibilities: Vec<ItemListPossibility>,
}


#[static_data_staging_table(knowledge_stat_modifier_desc)]
#[spacetimedb::table(name = knowledge_stat_modifier_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct KnowledgeStatModifierDesc {
    #[primary_key]
    pub secondary_knowledge_id: i32,
    pub stats: Vec<CsvStatEntry>,
}

#[static_data_staging_table(achievement_desc)]
#[spacetimedb::table(name = achievement_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct AchievementDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub requisites: Vec<i32>,
    pub skill_id: i32,
    pub skill_level: i32,
    pub resource_disc: Vec<i32>,
    pub crafting_disc: Vec<i32>,
    pub cargo_disc: Vec<i32>,
    pub item_disc: Vec<i32>,
    pub collectible_rewards: Vec<i32>,
    pub points_reward: i32,
    pub pct_chunks_discovered: f32,
    pub chunks_discovered: i32,
}

#[static_data_staging_table(alert_desc)]
#[spacetimedb::table(name = alert_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct AlertDesc {
    #[primary_key]
    pub alert_type: i32,
    pub show_timer: bool,
    pub duration: f32,
    pub header: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "AlertType")]
#[repr(i32)]
pub enum AlertType {
    None, // 0
    EvictionWarning,
    EvictionStatement,
    DismissableTest,
    OutOfSupplies,
    OutOfSuppliesInOneTick,
    OutOfSuppliesInTwelveTicks,
    NewLostItems,
    CoOwnerClaimOwnershipTransferIn24h,
    CoOwnerClaimOwnershipTransfer,
    OfficerClaimOwnershipTransfer,
    MemberClaimOwnershipTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "VfxAttachmentPoint")]
#[repr(u8)]
pub enum VfxAttachmentPoint {
    Origin,
    Overhead,
    Head,
    Chest,
    MainHand,
    OffHand,
    BothHands,
    Feet
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct FootprintTile {
    pub x: i32,
    pub z: i32,
    pub footprint_type: FootprintType,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct BuildingFunction {
    pub function_type: i32,
    pub level: i32,
    pub crafting_slots: i32,
    pub storage_slots: i32,
    pub cargo_slots: i32,
    pub refining_slots: i32,
    pub refining_cargo_slots: i32,
    pub item_slot_size: i32,
    pub cargo_slot_size: i32,
    pub trade_orders: i32,
    pub allowed_item_id_per_slot: Vec<i32>,
    pub buff_ids: Vec<i32>,
    pub concurrent_crafts_per_player: i32,
    pub terraform: bool,
    pub housing_slots: i32,
    pub housing_income: u32,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct ClothingVisual {
    pub meshe_names: Vec<String>,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct HandEquipmentVisual {
    pub main_hand: bool,
    pub prefab_names: Vec<String>,
}

#[derive(SpacetimeType)]
pub struct StatEntry {
    pub id: CharacterStatType,
    pub value: f32,
}

#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub struct CsvStatEntry {
    pub id: CharacterStatType,
    pub value: f32,
    pub is_pct: bool,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct EquipmentSlot {
    pub item: Option<ItemStack>,
    pub primary: EquipmentSlotType,
}

#[static_data_staging_table(biome_desc)]
#[spacetimedb::table(name = biome_desc, public, index(name = disallow_player_build, btree(columns = [disallow_player_build])))]
#[derive(Clone, PartialEq, Debug)]
pub struct BiomeDesc {
    #[primary_key]
    pub biome_type: u8,
    pub name: String,
    pub description: String,
    pub hazard_level: String,
    pub icon_address: String,
    pub disallow_player_build: bool,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct ChestLootRarity {
    pub rarity: i32,
    pub probability: f32,
}

#[spacetimedb::table(name = staged_claim_tech_desc)]
#[spacetimedb::table(name = claim_tech_desc, public, index(name = tier, btree(columns = [tier])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimTechDesc {
    #[primary_key]
    pub id: i32,
    pub description: String,
    pub tier: i32,
    pub supplies_cost: i32,
    pub research_time: i32,
    pub requirements: Vec<i32>,
    pub input: Vec<ItemStack>,
    pub members: i32,
    pub area: i32,
    pub supplies: i32,
    pub xp_to_mint_hex_coin: u32,
}

#[static_data_staging_table(claim_tech_desc_v2)]
#[spacetimedb::table(name = claim_tech_desc_v2, public, index(name = tier, btree(columns = [tier])))]
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimTechDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub tier: i32,
    pub tech_type: ClaimTechType,
    pub supplies_cost: i32,
    pub research_time: i32,
    pub requirements: Vec<i32>,
    pub input: Vec<ItemStack>,
    pub members: i32,
    pub area: i32,
    pub supplies: i32,
    pub xp_to_mint_hex_coin: u32,
    pub unlocks_techs: Vec<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "ClaimTechType")]
pub enum ClaimTechType {
    Undefined,

    //Generic techs for every tier
    TierUpgrade,
    MemberCount,
    MaxSupplies,
    ClaimArea,
    Settlement,
    
    //Professions
    Forestry,
    Carpentry,
    Masonry,
    Mining,
    Smithing,
    Scholar,
    Leatherworking,
    Hunting,
    Tailoring,
    Farming,
    Fishing,
    Foraging,

    //Special one-off techs
    Empire,
    TownBank,
    LargeHouse,
    TownMarket,
}

#[static_data_staging_table(climb_requirement_desc)]
#[spacetimedb::table(name = climb_requirement_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ClimbRequirementDesc {
    #[primary_key]
    #[auto_inc]
    pub id: i32,
    pub min_elevation: i16,
    pub max_elevation: i16,
    pub stamina_cost: i32,
    pub min_climb_proficiency: f32,
}

#[static_data_staging_table(onboarding_reward_desc)]
#[spacetimedb::table(name = onboarding_reward_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct OnboardingRewardDesc {
    #[primary_key]
    pub state_id: u16,
    pub item_stack_rewards: Vec<ItemStack>,
}

#[static_data_staging_table(wall_desc)]
#[spacetimedb::table(name = wall_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct WallDesc {
    #[primary_key]
    pub building_id: i32,
    pub large_post_asset_name: String,
    pub small_post_asset_name: String,
    pub wall_asset_names: Vec<String>,
}

#[static_data_staging_table(gate_desc)]
#[spacetimedb::table(name = gate_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct GateDesc {
    #[primary_key]
    pub building_id: i32,
    pub small_post_asset_name: String,
}

#[derive(SpacetimeType, Clone, PartialEq, Debug)]
pub struct PathfindingTraversalOption {
    pub max_elevation_difference: i32,
    pub move_type: PathfindingTraversalSettings,
}

#[static_data_staging_table(pathfinding_desc)]
#[spacetimedb::table(name = pathfinding_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PathfindingDesc {
    #[primary_key]
    pub id: i32,
    pub can_walk_on_land: bool,
    pub can_swim: bool,
    pub requires_transitions: bool,
    pub min_water_depth: i32,
    pub max_water_depth: i32,
    pub max_swim_height_delta: i32,
    pub avoid_light: bool,
    pub climb_up_options: Vec<PathfindingTraversalOption>,
    pub climb_down_options: Vec<PathfindingTraversalOption>,
}

#[static_data_staging_table(elevator_desc)]
#[spacetimedb::table(name = elevator_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ElevatorDesc {
    #[primary_key]
    pub building_id: i32,
    pub max_cliff_height: u32,
    pub speed: f32,
    pub usable_with_deployable: bool,
    pub top_platform_address: String,
    pub basket_address: String,
    pub bottom_platform_address: String,
}

#[static_data_staging_table(player_action_desc)]
#[spacetimedb::table(name = player_action_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PlayerActionDesc {
    #[primary_key]
    pub action_type_id: i32,
    pub layer: PlayerActionLayer,
    pub allowed_concurrent_action_ids: Vec<i32>,
}


#[static_data_staging_table(distant_visible_entity_desc)]
#[spacetimedb::table(name = distant_visible_entity_desc, public, index(name = description_id, btree(columns = [description_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct DistantVisibleEntityDesc {
    #[primary_key]
    pub id: i32,
    pub entity_type: EntityType,
    pub description_id: i32
}

#[static_data_staging_table(player_housing_desc)]
#[spacetimedb::table(name = player_housing_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PlayerHousingDesc {
    #[primary_key]
    pub secondary_knowledge_id: i32,
    pub rank: i32,
    pub name: String,
    pub template_building_id: i32,
}

#[static_data_staging_table(empire_colors_desc)]
#[spacetimedb::table(name = empire_color_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireColorDesc {
    #[primary_key]
    pub id: i32,
    pub color_argb: u64,
    pub color2_argb: Option<u64>,
}

#[static_data_staging_table(empire_icon_desc)]
#[spacetimedb::table(name = empire_icon_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct EmpireIconDesc {
    #[primary_key]
    pub id: i32,
    pub icon_unicode: String,
    pub is_shape : bool,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "EmpireNotificationType")]
#[repr(i32)]
pub enum EmpireNotificationType {
    None = 0,
    NewMember = 1,
    MarkedForSiege = 2,
    StartedSiege = 3,
    StartedDefense = 4,
    SuccessfulSiege = 5,
    SuccessfulDefense = 6,
    FailedSiege = 7,
    FailedDefense = 8,
    MemberLeft = 9,
    WatchtowerBuilt = 10,
    ClaimJoined = 11,
    ClaimLeft = 12,
    Donation = 13,
    DonationByProxy = 14,
}

#[static_data_staging_table(hexite_exchange_entry_desc)]
#[spacetimedb::table(name = hexite_exchange_entry_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct HexiteExchangeEntryDesc {
    #[primary_key]
    pub id: i32,
    pub image_address: String,
    pub shard_amount: u32,
    pub base_shard_amount: u32,
    pub name : String,
    pub price: f32,
    pub event_name: String,
}

#[static_data_staging_table(reserved_name_desc)]
#[spacetimedb::table(name = reserved_name_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ReservedNameDesc {
    #[primary_key]
    pub name: String,
}

#[static_data_staging_table(wind_params_desc)]
#[spacetimedb::table(name = wind_params_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct WindParamsDesc {
    #[primary_key]
    pub id: i32,
    pub scale: f64,
    pub weight: f64,
    pub cycle_sec: u64, //How long it takes noise to loop to original values (ie. walk a circle with radius of 10k units)
}

#[static_data_staging_table(premium_item_desc)]
#[spacetimedb::table(name = premium_item_desc, public, 
    index(name = collectible_desc_id, btree(columns = [collectible_desc_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct PremiumItemDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub image_address: String,
    pub collectible_desc_id: i32,
    pub price: u32,
    pub base_price: u32,
    #[default(false)]
    pub is_enabled: bool,
    #[default(1)]
    pub quantity: u32,
    #[default(0)]
    pub sorting_priority: u16,
}

#[static_data_staging_table(premium_service_desc)]
#[spacetimedb::table(name = premium_service_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct PremiumServiceDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub image_address: String,
    pub service_type: PremiumServiceType,
    pub price: u32,
    pub base_price: u32,
    #[default(false)]
    pub is_enabled: bool,
}

//TODO remove?
#[spacetimedb::table(name = wind_dbg_desc, public)]
pub struct WindDbgDesc {
    #[primary_key]
    pub id: i32,
    pub time_multiplier: f64,
}

#[static_data_staging_table(ability_unlock_desc)]
#[spacetimedb::table(name = ability_unlock_desc, public, index(name = ability_type_enum, btree(columns = [ability_type_enum_id])))]
#[derive(Clone, PartialEq, Debug)]
pub struct AbilityUnlockDesc {
    #[primary_key]
    pub id: i32,
    pub ability_type_enum_id: i32,                    // Matching AbilityTypeEnum for indexation
    pub ability_data: Option<AbilityType>,            // For specific ability + argument, like "AbilityType::CombatAction(5)" (shortsword_auto_attack). Use None for global coverage.
    pub level_requirements: Vec<LevelRequirement>,
    pub required_claim_tech_id: i32,
    pub required_knowledges: Vec<i32>,
    pub blocking_knowledges: Vec<i32>,
}

#[static_data_staging_table(ability_custom_desc)]
#[spacetimedb::table(name = ability_custom_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct AbilityCustomDesc {
    #[primary_key]
    pub id: i32,
    pub ability_name: String,
    pub stamina_cost: i32,
    pub health_cost: i32,
    pub cast_time: f32,                     // 0.0 for instant cast
    pub range: f32,                         // target range, 0 for self
    pub friendly: bool,                     // targets players if true, enemies if false
    pub radius: f32,                        // [NOT IMPLEMENTED YET] area of effect around the target
    pub buffs: Vec<BuffEffect>,             // buffs to apply
    pub buff_toggle: bool,                  // deactivate the specified buffs in they are active
    pub damage: i32,                        // positive for damage, negative for healing
    pub threat_value: f32,                  // threat caused by this action
    pub cooldown: f32,
    pub global_cooldown: f32,
    pub linked_ability_buff_desc_id: i32,   // if need to trigger another entry for compound effect
    pub animation: String,                  // animation string for animator
    pub icon_path: String,                  // for action bar ui
}

#[static_data_staging_table(prospecting_desc)]
#[spacetimedb::table(name = prospecting_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct ProspectingDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,                       // for UI
    pub description: String,
    pub resource_clump_id: i32,             // spawns either a resource or a herd
    pub placeholder_resource_clump_id: i32, // spawns this resource when the trail is created to hold the position
    pub enemy_ai_desc_id: i32,              // spawns either a resource or a herd
    pub is_aquatic_resource: bool,           // spawn in water is defined by world generation, which prospecting resources aren't part of

    pub biome_requirements: Vec<i32>,                // biomes in which it can be triggered

    pub equipment_requirements: Vec<i32>,            // Trinkets etc., not used at the moment

    pub required_items_to_start: Vec<ItemStack>,
    pub required_items_to_interact_with_reward: Vec<ItemStack>,
    pub consumed_items_by_ability_trigger: Vec<ItemStack>,
    pub allow_aquatic_prospecting: bool, 
    
    pub bread_crumb_count: Vec<i32>,             // # of steps (min, max) for the treasure hunt
    pub bread_crumb_radius: Vec<i32>,       // (min, max) in large tiles
    pub distance_between_bread_crumbs: Vec<i32>,    // in large tiles
    pub deadzone_angle_between_crumbs: f32,       // in degrees
    pub allow_aquatic_bread_crumb: bool,          // bread crumb can appear in water
    pub pointer_duration: f32,                  // seconds during which the pointer to the next crumb shows after moving
    pub prospecting_duration: f32,              // animation/vfx time before action applies

    pub join_radius: i32,                   // in large tiles, always centered on the active crumb
    pub contribution_per_visited_bread_crumb: i32,         // number of hits on prize per visited node

    pub breadcrumb_found_message: String,
    pub resource_uncovered_message: String,
    pub breadcrumb_found_by_someone_else_message: String,
    pub resource_uncovered_by_someone_else_message: String,

    pub icon_asset_path: String,

}

#[static_data_staging_table(quest_chain_desc)]
#[spacetimedb::table(name = quest_chain_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct QuestChainDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub is_hint: bool,
    pub stages: Vec<i32>,
    pub requirements: Vec<QuestRequirement>,
    pub rewards: Vec<QuestReward>,
    pub implicit_rewards: Vec<QuestReward>, // Doesn't get awarded, but you end up getting as a result of following the quest (e.g. getting a cart on the cart quest)
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
#[sats(name = "QuestRequirement")]
pub enum QuestRequirement {
    PaddingNone(ColumnPad4u64),
    QuestChain(i32),
    Achievement(i32),
    Collectible(i32),
    Level(LevelRequirement),
    ItemStack(ItemStack),
    SecondaryKnowledge(i32),
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
#[sats(name = "QuestReward")]
pub enum QuestReward {
    PaddingNone(ColumnPad4u64),
    ItemStack(ItemStack),
    Achievement(i32),
    Collectible(i32),
    Experience(ExperienceStackF32),
    SecondaryKnowledge(i32),
}

#[static_data_staging_table(stage_rewards_desc)]
#[spacetimedb::table(name = stage_rewards_desc)]
#[derive(Clone, PartialEq, Debug)]
pub struct StageRewardsDesc {
    #[primary_key]
    pub id : i32,
    pub chain_desc_id : i32,
    pub rewards: Vec<ItemStack>,
}

#[static_data_staging_table(quest_stage_desc)]
#[spacetimedb::table(name = quest_stage_desc, public)]
#[derive(Clone, PartialEq, Debug)]
pub struct QuestStageDesc {
    #[primary_key]
    pub id: i32,
    pub chain_desc_id : i32,
    pub name: String,
    pub completion_conditions: Vec<CompletionCondition>,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
#[sats(name = "CompletionCondition")]
pub enum CompletionCondition {
    PaddingNone(ColumnPad4u64),
    ItemStack(ItemStackCompletionCondition),
    Achievement(i32),
    Collectible(i32),
    Level(LevelRequirement),
    SecondaryKnowledge(i32),
    EquippedItem(i32)
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ItemStackCompletionCondition {
    pub item_stack : ItemStack,
    pub is_consumed : bool,
}

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ColumnPad4u64 {
    // Not pretty, but it makes room for larger enums in the future.
    pub pad0 : u64,
    pub pad1 : u64,
    pub pad2 : u64,
    pub pad3 : u64,
}