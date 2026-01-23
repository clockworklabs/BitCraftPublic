# BitCraft Data Models

## Table of Contents

- [Overview](#overview)
- [Core Entity Tables](#core-entity-tables)
- [Player System Tables](#player-system-tables)
- [Inventory System Tables](#inventory-system-tables)
- [Building System Tables](#building-system-tables)
- [Claim System Tables](#claim-system-tables)
- [Empire System Tables](#empire-system-tables)
- [Combat System Tables](#combat-system-tables)
- [Crafting System Tables](#crafting-system-tables)
- [World and Environment Tables](#world-and-environment-tables)
- [Static Data Tables](#static-data-tables)
- [Utility Tables](#utility-tables)

## Overview

BitCraft uses **SpacetimeDB tables** as its primary data storage mechanism. Tables are defined as Rust structs with the `#[spacetimedb::table]` attribute.

### Table Characteristics

- **~200 total tables** across both game and global modules
- **Public tables**: Visible to clients (marked with `public` attribute)
- **Primary keys**: Automatic B-tree indexes for fast lookups
- **Additional indexes**: Custom indexes on frequently queried fields
- **Auto-increment**: Some tables use `#[auto_inc]` for automatic ID generation

### Table Naming Conventions

- **`*State`** - Runtime state that changes during gameplay
- **`*Desc`** - Static descriptive data (items, buildings, recipes, etc.)
- **`*Schedule`** - Agent scheduling tables
- **`*Log`** - Historical data and audit trails

## Core Entity Tables

### LocationState

Stores the base location for all positioned entities.

```rust
#[spacetimedb::table(name = location_state, public)]
pub struct LocationState {
    #[primary_key]
    pub entity_id: u64,
    pub location: SmallHexTile,  // Hex tile coordinates (q, r, s)
}
```

**Used By**: All positioned entities (players, buildings, resources, enemies, etc.)

### MobileEntityState

Extended location data for entities that can move.

```rust
#[spacetimedb::table(name = mobile_entity_state, public)]
pub struct MobileEntityState {
    #[primary_key]
    pub entity_id: u64,
    pub location_x: f32,           // Precise X position
    pub location_z: f32,           // Precise Z position
    pub dimension: u32,            // Current dimension
    pub destination_x: f32,        // Target X for movement
    pub destination_z: f32,        // Target Z for movement
    pub is_running: bool,          // Walk vs run speed
    pub chunk_index: u64,          // Current chunk
    pub timestamp: u64,            // Last update time
}
```

**Used By**: Players, enemies, NPCs

### HealthState

Health tracking for damageable entities.

```rust
#[spacetimedb::table(name = health_state, public)]
pub struct HealthState {
    #[primary_key]
    pub entity_id: u64,
    pub health: f32,
    pub max_health: f32,
}
```

**Used By**: Players, enemies, buildings, resources

### StaminaState

Stamina system for players.

```rust
#[spacetimedb::table(name = stamina_state, public)]
pub struct StaminaState {
    #[primary_key]
    pub entity_id: u64,
    pub stamina: f32,
    pub max_stamina: f32,
    pub stamina_regen_rate: f32,
}
```

**Used By**: Players

## Player System Tables

### PlayerState

Core player data and session information.

```rust
#[spacetimedb::table(name = player_state, public)]
pub struct PlayerState {
    #[primary_key]
    pub entity_id: u64,

    // Session tracking
    pub signed_in: bool,
    pub session_start_timestamp: u64,
    pub sign_in_timestamp: u64,
    pub previous_sign_in_timestamp: u64,
    pub last_action_timestamp: u64,

    // Death and respawn
    pub time_since_last_death_in_ms: u64,
    pub death_count: i32,

    // Teleportation
    pub teleport_location: TeleportLocation,
    pub home_location: Option<SmallHexTile>,
    pub home_dimension: u32,

    // Gameplay state
    pub is_incapacitated: bool,
    pub food_level: f32,
    pub comfort_level: f32,

    // Progression
    pub total_play_time_in_ms: u64,
    pub level: i32,
}

pub enum TeleportLocation {
    None,
    Home,
    Waystone(u64),  // Building entity ID
    BirthLocation,
}
```

### PlayerUsernameState

Player names and display information.

```rust
#[spacetimedb::table(name = player_username_state, public)]
pub struct PlayerUsernameState {
    #[primary_key]
    pub entity_id: u64,
    pub username: String,
    pub display_name: String,
    pub title: String,
}
```

**Note**: Replicated to global module for empire/social features

### PlayerActionState

Tracks current player actions.

```rust
#[spacetimedb::table(name = player_action_state, public)]
pub struct PlayerActionState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub entity_id: u64,
    pub player_action: PlayerAction,
    pub player_action_layer: PlayerActionLayer,
    pub timestamp: u64,
}

pub enum PlayerAction {
    Idle,
    Attack(u64),          // Target entity ID
    Build(i32),           // Building description ID
    Craft(i32),           // Recipe ID
    Extract(u64),         // Resource entity ID
    Move,
    Teleport,
    Sleep,
    Eat,
    UseItem(i32),         // Item ID
    Climb,
    Death,
    Emote(i32),           // Emote ID
    Trade,
    // ... 23 total action types
}

pub enum PlayerActionLayer {
    Base,      // Primary action
    Secondary, // Concurrent action (e.g., emote while moving)
    Tertiary,  // Additional layer
}
```

### SignedInPlayerState

Fast lookup table for active players.

```rust
#[spacetimedb::table(name = signed_in_player_state, public)]
pub struct SignedInPlayerState {
    #[primary_key]
    pub entity_id: u64,
    pub identity: Identity,  // SpacetimeDB client identity
}
```

### UserState

User account and authentication.

```rust
#[spacetimedb::table(name = user_state, public)]
pub struct UserState {
    #[primary_key]
    pub identity: Identity,
    pub entity_id: u64,
    pub can_sign_in: bool,
    pub queue_position: i32,
    pub grace_period_end: u64,
    pub roles: Vec<Role>,
}

pub enum Role {
    Admin,
    GM,
    SkipQueue,
    Developer,
    Tester,
}
```

### ExperienceState

Player skill progression.

```rust
#[spacetimedb::table(name = experience_state, public)]
pub struct ExperienceState {
    #[primary_key]
    pub entity_id: u64,
    pub experience_stacks: Vec<ExperienceStack>,
}

pub struct ExperienceStack {
    pub skill_id: i32,
    pub quantity: i32,  // Total XP in this skill
}
```

**Skills**: Mining, Woodcutting, Combat, Crafting, Building, Cooking, etc.

## Inventory System Tables

### InventoryState

Multi-pocket inventory system.

```rust
#[spacetimedb::table(name = inventory_state, public)]
pub struct InventoryState {
    #[primary_key]
    pub entity_id: u64,
    pub pockets: Vec<Pocket>,
    pub cargo_index: i32,           // Index of cargo pocket
    pub inventory_index: i32,       // Index of main inventory
    pub owner_entity_id: u64,       // Owner (for building inventories)
    pub player_owner_entity_id: u64,
}

pub struct Pocket {
    pub item_stacks: Vec<ItemStack>,
    pub volume: i32,       // Max volume
    pub locked: bool,      // Can't be modified
}

pub struct ItemStack {
    pub item_id: i32,
    pub quantity: i32,
}
```

**Pockets**: Different inventory sections (main, cargo, equipment, etc.)

### EquipmentState

Equipped items and loadouts.

```rust
#[spacetimedb::table(name = equipment_state, public)]
pub struct EquipmentState {
    #[primary_key]
    pub entity_id: u64,
    pub equipment_slots: Vec<EquipmentSlot>,
}

pub struct EquipmentSlot {
    pub slot_id: i32,
    pub item_id: i32,
    pub durability: f32,
}
```

**Equipment Slots**: Weapon, Armor, Tools, Accessories

### VaultState

Personal bank storage.

```rust
#[spacetimedb::table(name = vault_state, public)]
pub struct VaultState {
    #[primary_key]
    pub entity_id: u64,
    pub vault_pockets: Vec<Pocket>,
    pub unlocked_tabs: Vec<i32>,
}
```

### DroppedInventoryState

Items dropped on the ground.

```rust
#[spacetimedb::table(name = dropped_inventory_state, public)]
pub struct DroppedInventoryState {
    #[primary_key]
    pub entity_id: u64,
    pub dropped_by_entity_id: u64,
    pub items: Vec<ItemStack>,
    pub drop_timestamp: u64,
    pub protected_until: u64,      // Time before others can pick up
    pub despawn_at: u64,           // Auto-cleanup time
    pub is_death_drop: bool,       // Dropped on death
}
```

## Building System Tables

### BuildingState

Placed building instances.

```rust
#[spacetimedb::table(name = building_state, public)]
pub struct BuildingState {
    #[primary_key]
    pub entity_id: u64,
    pub building_description_id: i32,  // References BuildingDesc
    pub direction_index: u8,           // Rotation (0-5 for hex)
    pub claim_entity_id: u64,          // Owning claim
    pub network_entity_id: u64,        // Storage network ID
}
```

### BuildingDesc

Static building definitions.

```rust
#[spacetimedb::table(name = building_desc)]
pub struct BuildingDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub tier: i32,
    pub category: BuildingCategory,
    pub functions: Vec<BuildingFunctionDesc>,
    pub footprint: Vec<FootprintDelta>,
    pub maintenance: f32,           // Upkeep cost
    pub max_health: f32,
    pub construction_time_ms: u64,
    pub building_value: i32,        // Territory value
}

pub enum BuildingCategory {
    Workshop, Storage, Bank, Forge, Portal, Waystone,
    Marketplace, ClaimTotem, Housing, Farm, Mine,
    // ... 30+ categories
}

pub enum BuildingFunctionDesc {
    Storage { volume: i32 },
    Crafting { recipes: Vec<i32> },
    Teleportation,
    Bank,
    Spawn { resource_id: i32, rate_ms: u64 },
    Portal { destination: PortalLocationDesc },
    // ... many more functions
}
```

### ProjectSiteState

Buildings under construction.

```rust
#[spacetimedb::table(name = project_site_state, public)]
pub struct ProjectSiteState {
    #[primary_key]
    pub entity_id: u64,
    pub building_description_id: i32,
    pub direction_index: u8,
    pub claim_entity_id: u64,
    pub materials_contributed: Vec<ItemStack>,
    pub materials_required: Vec<ItemStack>,
    pub construction_progress: f32,  // 0.0 to 1.0
    pub is_public: bool,             // Can anyone contribute?
}
```

### BuildingNicknameState

Custom building names and signs.

```rust
#[spacetimedb::table(name = building_nickname_state, public)]
pub struct BuildingNicknameState {
    #[primary_key]
    pub entity_id: u64,
    pub nickname: String,
    pub sign_text: String,
}
```

### StorageNetworkState

Linked storage systems.

```rust
#[spacetimedb::table(name = storage_network_state, public)]
pub struct StorageNetworkState {
    #[primary_key]
    pub entity_id: u64,
    pub building_entity_ids: Vec<u64>,  // Connected storages
    pub total_volume: i32,
}
```

## Claim System Tables

### ClaimState

Territory ownership.

```rust
#[spacetimedb::table(name = claim_state, public)]
pub struct ClaimState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,       // Owner player
    pub empire_entity_id: u64,      // Parent empire (if any)
    pub claim_type: ClaimType,
    pub founding_timestamp: u64,
    pub supplies: f32,              // Current supply level
    pub max_supplies: f32,
    pub protection_threshold: f32,  // Supplies needed for protection
}

pub enum ClaimType {
    Personal,
    Clan,
    Settlement,
    Outpost,
}
```

### ClaimMemberState

Claim membership and permissions.

```rust
#[spacetimedb::table(name = claim_member_state, public)]
pub struct ClaimMemberState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub claim_entity_id: u64,
    pub player_entity_id: u64,
    pub permissions: ClaimPermissions,
    pub join_timestamp: u64,
}

pub struct ClaimPermissions {
    pub inventory: bool,
    pub build: bool,
    pub usage: bool,
    pub recruit: bool,
    pub remove_member: bool,
    pub edit_permissions: bool,
    pub add_tile: bool,
    pub remove_tile: bool,
}
```

### ClaimTileState

Territory tiles.

```rust
#[spacetimedb::table(name = claim_tile_state, public)]
pub struct ClaimTileState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub claim_entity_id: u64,
    pub location: SmallHexTile,
    pub added_timestamp: u64,
}
```

### ClaimTechState

Claim technology tree.

```rust
#[spacetimedb::table(name = claim_tech_state, public)]
pub struct ClaimTechState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub claim_entity_id: u64,
    pub tech_id: i32,
    pub unlocked: bool,
    pub research_progress: f32,
}
```

### ClaimDescriptionState

Claim display information.

```rust
#[spacetimedb::table(name = claim_description_state, public)]
pub struct ClaimDescriptionState {
    #[primary_key]
    pub entity_id: u64,
    pub name: String,
    pub description: String,
    pub emblem_icon_id: i32,
    pub emblem_color: Color,
}
```

## Empire System Tables

These tables are in the **Global Module** and replicated to regions.

### EmpireState

Empire core data.

```rust
#[spacetimedb::table(name = empire_state, public)]
pub struct EmpireState {
    #[primary_key]
    pub entity_id: u64,
    pub name: String,
    pub founded_timestamp: u64,
    pub emperor_entity_id: u64,     // Current emperor
    pub supply_level: f32,           // Empire-wide supplies
    pub territory_count: i32,
    pub member_count: i32,
    pub rank_count: i32,
}
```

### EmpirePlayerDataState

Empire membership.

```rust
#[spacetimedb::table(name = empire_player_data_state, public)]
pub struct EmpirePlayerDataState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub empire_entity_id: u64,
    pub player_entity_id: u64,
    pub rank_index: i32,
    pub join_timestamp: u64,
    pub contribution_points: i32,
}
```

### EmpireRankState

Rank system with permissions.

```rust
#[spacetimedb::table(name = empire_rank_state, public)]
pub struct EmpireRankState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub empire_entity_id: u64,
    pub rank_index: i32,
    pub rank_title: String,
    pub permissions: EmpirePermissions,
}

pub struct EmpirePermissions {
    pub invite: bool,
    pub expel: bool,
    pub manage_ranks: bool,
    pub manage_settlements: bool,
    pub declare_war: bool,
    pub manage_treasury: bool,
    pub mark_expansion: bool,
    pub start_siege: bool,
    // ... 14 total permissions
}
```

### EmpireSettlementState

Settlements (claims) within empire.

```rust
#[spacetimedb::table(name = empire_settlement_state, public)]
pub struct EmpireSettlementState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub empire_entity_id: u64,
    pub claim_entity_id: u64,
    pub region_index: u32,
    pub settlement_type: SettlementType,
    pub joined_timestamp: u64,
}

pub enum SettlementType {
    Capital,
    Settlement,
    Outpost,
}
```

### EmpireSiegeState

PvP siege system.

```rust
#[spacetimedb::table(name = empire_siege_state, public)]
pub struct EmpireSiegeState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub target_settlement_id: u64,
    pub attacking_empire_id: u64,
    pub defending_empire_id: u64,
    pub siege_start_timestamp: u64,
    pub siege_end_timestamp: u64,
    pub attacker_supplies: f32,
    pub defender_supplies: f32,
    pub status: SiegeStatus,
}

pub enum SiegeStatus {
    Pending,
    Active,
    AttackerVictory,
    DefenderVictory,
    Cancelled,
}
```

### EmpireTerritoryState

Territorial control.

```rust
#[spacetimedb::table(name = empire_territory_state, public)]
pub struct EmpireTerritoryState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub empire_entity_id: u64,
    pub territory_tiles: Vec<SmallHexTile>,
    pub region_index: u32,
}
```

## Combat System Tables

### CombatState

Active combat status.

```rust
#[spacetimedb::table(name = combat_state, public)]
pub struct CombatState {
    #[primary_key]
    pub entity_id: u64,
    pub in_combat: bool,
    pub combat_start_timestamp: u64,
    pub last_combat_action_timestamp: u64,
}
```

### AttackOutcomeState

Damage results.

```rust
#[spacetimedb::table(name = attack_outcome_state, public)]
pub struct AttackOutcomeState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub attacker_entity_id: u64,
    pub defender_entity_id: u64,
    pub damage: f32,
    pub is_critical: bool,
    pub dodged: bool,
    pub timestamp: u64,
}
```

### ThreatState

Enemy aggro system.

```rust
#[spacetimedb::table(name = threat_state, public)]
pub struct ThreatState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub enemy_entity_id: u64,
    pub player_entity_id: u64,
    pub threat_level: f32,
    pub last_update_timestamp: u64,
}
```

### CombatActionDesc

Attack definitions.

```rust
#[spacetimedb::table(name = combat_action_desc)]
pub struct CombatActionDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub damage: f32,
    pub attack_speed_ms: u64,
    pub range: f32,
    pub stamina_cost: f32,
    pub combo_count: i32,        // Multi-hit attacks
    pub status_effects: Vec<StatusEffectDesc>,
}
```

### BuffState

Active buffs/debuffs.

```rust
#[spacetimedb::table(name = buff_state, public)]
pub struct BuffState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub entity_id: u64,
    pub buff_id: i32,
    pub stacks: i32,
    pub applied_timestamp: u64,
    pub expires_at: u64,
}
```

## Crafting System Tables

### CraftingRecipeDesc

Recipe definitions.

```rust
#[spacetimedb::table(name = crafting_recipe_desc)]
pub struct CraftingRecipeDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub recipe_type: RecipeType,
    pub inputs: Vec<ItemStack>,     // Materials required
    pub outputs: Vec<ItemStack>,    // Items produced
    pub tool_requirement: Option<i32>,
    pub building_requirement: Option<i32>,
    pub skill_requirement: Option<SkillRequirement>,
    pub tech_requirement: Option<i32>,
    pub craft_time_ms: u64,
}

pub enum RecipeType {
    Crafting,        // Active crafting
    Construction,    // Building placement
    Extraction,      // Resource harvesting
    ItemConversion,  // Transform items
    Terraform,       // Modify terrain
    Growth,          // Plant/grow resources
}
```

### ProgressiveActionState

Active crafting/construction.

```rust
#[spacetimedb::table(name = progressive_action_state, public)]
pub struct ProgressiveActionState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub recipe_id: i32,
    pub building_entity_id: u64,
    pub lock_expiration: u64,
    pub items_completed: i32,
    pub items_requested: i32,
    pub suspended_timestamp: u64,
    pub status: ProgressiveActionStatus,
}
```

### PassiveCraftState

Queued passive crafting.

```rust
#[spacetimedb::table(name = passive_craft_state, public)]
pub struct PassiveCraftState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub recipe_id: i32,
    pub queue_position: i32,
    pub items_crafted: i32,
    pub items_requested: i32,
    pub start_timestamp: u64,
    pub estimated_completion: u64,
}
```

## World and Environment Tables

### TerrainChunkState

Terrain data storage.

```rust
#[spacetimedb::table(name = terrain_chunk_state, public)]
pub struct TerrainChunkState {
    #[primary_key]
    pub chunk_index: u64,
    pub terrain_nodes: Vec<TerrainNode>,
}

pub struct TerrainNode {
    pub elevation: f32,
    pub water_level: f32,
    pub biome: Biome,
    pub is_passable: bool,
}
```

### ResourceState

Harvestable resources.

```rust
#[spacetimedb::table(name = resource_state, public)]
pub struct ResourceState {
    #[primary_key]
    pub entity_id: u64,
    pub resource_description_id: i32,
    pub respawn_timestamp: u64,
}
```

### ResourceDesc

Resource definitions.

```rust
#[spacetimedb::table(name = resource_desc)]
pub struct ResourceDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub extraction_recipe_id: i32,
    pub max_health: f32,
    pub respawn_time_ms: u64,
    pub biomes: Vec<Biome>,      // Where it spawns
    pub footprint: Vec<FootprintDelta>,
}
```

### EnemyState

Enemy NPCs.

```rust
#[spacetimedb::table(name = enemy_state, public)]
pub struct EnemyState {
    #[primary_key]
    pub entity_id: u64,
    pub enemy_description_id: i32,
    pub ai_state: EnemyAIState,
    pub home_location: SmallHexTile,
    pub herd_entity_id: u64,
    pub level: i32,
}

pub enum EnemyAIState {
    Idle,
    Wandering,
    Fighting,
    Fleeing,
    Retreating,
    Dead,
}
```

### HerdState

Enemy group behavior.

```rust
#[spacetimedb::table(name = herd_state, public)]
pub struct HerdState {
    #[primary_key]
    pub entity_id: u64,
    pub herd_leader_entity_id: u64,
    pub member_entity_ids: Vec<u64>,
    pub center_location: SmallHexTile,
    pub wander_radius: f32,
}
```

### BiomeDesc

Biome definitions.

```rust
#[spacetimedb::table(name = biome_desc)]
pub struct BiomeDesc {
    #[primary_key]
    pub biome: Biome,
    pub name: String,
    pub elevation_min: f32,
    pub elevation_max: f32,
    pub moisture_min: f32,
    pub moisture_max: f32,
    pub temperature_min: f32,
    pub temperature_max: f32,
}

pub enum Biome {
    Ocean, CalmForest, PineWoods, SnowyPeaks,
    BreezyPlains, AutumnForest, Tundra, Desert,
    Swamp, Canyon, Cave, Jungle, Sapwoods, SafeMeadows,
}
```

## Static Data Tables

### ItemDesc

Item definitions (500+ items).

```rust
#[spacetimedb::table(name = item_desc)]
pub struct ItemDesc {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub description: String,
    pub item_type: ItemType,
    pub max_stack_size: i32,
    pub volume: i32,
    pub value: i32,
    pub durability: Option<f32>,
    pub stats: ItemStats,
}

pub enum ItemType {
    Resource, Tool, Weapon, Armor, Food,
    Consumable, Ingredient, Furniture, Collectible,
    // ... many more types
}

pub struct ItemStats {
    pub damage: f32,
    pub armor: f32,
    pub speed_modifier: f32,
    pub durability_max: f32,
    // ... more stats
}
```

### ParametersDescV2

Global game parameters.

```rust
#[spacetimedb::table(name = parameters_desc_v2)]
pub struct ParametersDescV2 {
    #[primary_key]
    pub id: i32,
    pub name: String,
    pub value: f32,
}
```

**Examples**:
- `player_max_health`
- `stamina_regen_rate`
- `claim_supply_decay_rate`
- `building_decay_rate`
- Hundreds of balance parameters

## Utility Tables

### Globals

Global state and counters.

```rust
#[spacetimedb::table(name = globals, public)]
pub struct Globals {
    #[primary_key]
    pub version: u32,
    pub entity_pk_counter: u64,     // Next entity ID
    pub world_generated: bool,
    pub agents_enabled: bool,
    pub server_start_timestamp: u64,
}
```

### InterModuleMessageV3

Cross-module communication.

```rust
#[spacetimedb::table(name = inter_module_message_v3)]
pub struct InterModuleMessageV3 {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub destination: InterModuleDestination,
    pub sender_module_identity: Identity,
    pub contents: MessageContentsV3,
    pub timestamp: u64,
}
```

### ChatMessageState

In-game chat.

```rust
#[spacetimedb::table(name = chat_message_state, public)]
pub struct ChatMessageState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub sender_entity_id: u64,
    pub channel: ChatChannel,
    pub message: String,
    pub timestamp: u64,
}

pub enum ChatChannel {
    Local,
    Claim,
    Empire,
    Global,
    Whisper(u64),  // Target player
}
```

### AchievementState

Player achievements.

```rust
#[spacetimedb::table(name = achievement_state, public)]
pub struct AchievementState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub player_entity_id: u64,
    pub achievement_id: i32,
    pub progress: i32,
    pub completed: bool,
    pub completed_timestamp: u64,
}
```

### QuestState

Quest progression.

```rust
#[spacetimedb::table(name = quest_state, public)]
pub struct QuestState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub player_entity_id: u64,
    pub quest_id: i32,
    pub status: QuestStatus,
    pub objectives_completed: Vec<bool>,
    pub accepted_timestamp: u64,
}

pub enum QuestStatus {
    Available,
    Active,
    Completed,
    Failed,
}
```

## Agent Schedule Tables

Each agent has a schedule table for timing:

```rust
#[spacetimedb::table(scheduled(player_regen_agent, at = scheduled_at))]
pub struct PlayerRegenSchedule {
    pub scheduled_id: u64,
    pub scheduled_at: Timestamp,
}
```

**23 Schedule Tables** for different agents (regen, decay, AI, etc.)

## Summary

- **~200 tables** total across both modules
- **Entity-centric design** with shared `entity_id` keys
- **Separate State and Desc tables** for runtime vs static data
- **Public tables** for client visibility
- **Indexed queries** for performance
- **Comprehensive coverage** of all game systems

## Next Steps

- **[Reducers API](reducers-api.md)** - How to interact with these tables
- **[Game Systems](game-systems.md)** - How systems use these tables
- **[Architecture](architecture.md)** - Design patterns and relationships
