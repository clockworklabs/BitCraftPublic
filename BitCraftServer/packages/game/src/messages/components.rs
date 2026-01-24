use bitcraft_macro::*;
use spacetimedb::SpacetimeType;
use spacetimedb::{Identity, Timestamp};

use crate::messages::game_util::{ActiveBuff, ExperienceStack, ExperienceStackF32, ItemStack, Pocket, TradePocket};
use crate::messages::static_data::EquipmentSlot;
use crate::messages::util::*;
use crate::{BuffCategory, SkillType};

use super::game_util::DimensionType;
use super::static_data::{AlertType, EnemyType, FootprintType, NpcType};

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "EnemyStatus")]
#[repr(u8)]
pub enum EnemyStatus {
    Inactive,        // No player around and within herd radius, skip.
    Idle,            // Within herd radius but unaware of players around.
    ReturningToIdle, // Returning to last idle/inactive point - then will switch to inactive or idle
    Evasive,         // Idle, but favoring directions away from threats
    Investigating,   // Idle, but favoring direction towards targets
    Fighting,        // In combat. Will chase current target if it runs away.
    Retreating,      // Actively running away, might leave herd radius to find safety.
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "ProgressiveActionStatus")]
#[repr(u8)]
pub enum ProgressiveActionStatus {
    None,
    Active,
    Suspended,
    Completed,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PassiveCraftStatus")]
#[repr(u8)]
pub enum PassiveCraftStatus {
    Queued,
    Processing,
    Complete,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "TradeSessionStatus")]
#[repr(u8)]
pub enum TradeSessionStatus {
    SessionOffered,
    SessionAccepted,
    InitiatorAccepted,
    AcceptorAccepted,
    SessionResolved,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "ClaimPermission")]
#[repr(u8)]
pub enum ClaimPermission {
    Inventory,
    Build,
    Usage,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "KnowledgeState")]
#[repr(u8)]
pub enum KnowledgeState {
    Unknown,
    Discovered,
    Acquired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, spacetimedb::SpacetimeType)]
#[sats(name = "TeleportLocationType")]
#[repr(u8)]
pub enum TeleportLocationType {
    BirthLocation,
    TradingPost,
    HomeLocation,
    CustomLocation,
    Waystone,
}

impl Default for TeleportLocationType {
    fn default() -> Self {
        TeleportLocationType::BirthLocation
    }
}

use strum_macros::EnumIter;
#[derive(spacetimedb::SpacetimeType, Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, PartialOrd, Ord, Hash)]
#[sats(name = "Biome")]
#[repr(u8)]
pub enum Biome {
    #[default]
    Dev,
    CalmForest,
    PineWoods,
    SnowyPeaks,
    BreezyPlains,
    AutumnForest,
    Tundra,
    Desert,
    Swamp,
    Canyon,
    Ocean,
    SafeMeadows,
    Cave,
    Jungle,
    Sapwoods,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, Debug, Default, PartialEq, Eq, EnumIter, PartialOrd, Ord, Hash)]
#[sats(name = "SurfaceType")]
#[repr(u8)]
pub enum SurfaceType {
    #[default]
    Ground,
    Lake,
    River,
    Ocean,
    OceanBiome,
    Swamp,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PlayerActionType")]
#[repr(u8)]
pub enum PlayerActionType {
    None,
    Attack,
    DestroyPaving,
    StationaryEmote,
    Extract,
    PaveTile,
    SpawnCargo,
    Build,
    Deconstruct,
    RepairBuilding,
    ResupplyClaim,
    CargoPickUp,
    Terraform,
    DeployDeployable,
    StoreDeployable,
    Sleep,
    Teleport,
    Death,
    Climb,
    UseItem,
    Craft,
    ConvertItems,
    PlayerMove,
    DeployableMove,
    ResupplyEmpireNode,
    SetHome,
    UseElevator,
    MobileEmote,
    PlacePillarShaping,
    DestroyPillarShaping,
    AbilityCustom,
    Prospect,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PlayerActionResult")]
#[repr(u8)]
pub enum PlayerActionResult {
    Success,
    TimingFail,
    Fail,
    Cancel,
}

#[derive(spacetimedb::SpacetimeType, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UserModerationPolicy {
    PermanentBlockLogin,
    TemporaryBlockLogin,
    BlockChat,
    BlockConstruct,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "ChatChannel")]
#[repr(i32)]
pub enum ChatChannel { // It would be best to rename, since we have ChatChannelState in global module now.
    System,
    Global, // Not used
    Local,
    Region, // Called General in game
    Claim, // Not used
    EmpirePublic, // Not used
    EmpireInternal, // Not used
    LookingForGroup,
    Trade
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "Permission")]
#[repr(i32)]
pub enum Permission { // HousingPermission
    PendingVisitor,
    Visitor,
    Usage,
    Inventory,
    Build,
    CoOwner,
    OverrideNoAccess,   // Only Owner is stronger than OverrideNoAccess
    Owner,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PermissionGroup")]
#[repr(i32)]
pub enum PermissionGroup {
    Player = 0,
    Claim,
    Empire,
    Everyone,
}

#[spacetimedb::table(name = location_state, public,
    index(name = x_z_chunk_index, btree(columns = [x, z, chunk_index])),
    index(name = chunk_index, btree(columns = [chunk_index]))
    //DO NOT add dimension index - it leads to massive perf degradation. Use dimension_filter() and dimension_delete() instead
)]
#[shared_table]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)] // IMPORTANT: LOCATION SHOULD NOT HAVE THE COMMIT ATTRIBUTE
#[repr(C)]
pub struct LocationState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub chunk_index: u64,

    pub x: i32,
    pub z: i32,
    pub dimension: u32,
}
// Ensure that we don't have hidden padding in the struct
const _: () = assert!(size_of::<LocationState>() == 32);

#[spacetimedb::table(name = mobile_entity_state, public, 
    index(name = chunk_index, btree(columns = [chunk_index])),
    index(name = dimension, btree(columns = [dimension]))
)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)] // IMPORTANT: MOBILE_ENTITIES SHOULD NOT HAVE THE COMMIT ATTRIBUTE
#[repr(C)]
pub struct MobileEntityState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub chunk_index: u64,
    pub timestamp: u64,
    pub location_x: i32,
    pub location_z: i32,
    pub destination_x: i32,
    pub destination_z: i32,
    pub dimension: u32,
    pub is_running: bool,
    // Manual padding to make the struct exactly 48 bytes long.
    // SpacetimeDB can optimize row (de)serialization when:
    // 1. Table types are naturally aligned
    // 2. They do not have any internal padding
    pub _pad1: u8,
    pub _pad2: u8,
    pub _pad3: u8,
}
// Ensure that we don't have hidden padding in the struct
const _: () = assert!(size_of::<MobileEntityState>() == 48);

#[spacetimedb::table(name = move_validation_strike_counter_state)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct MoveValidationStrikeCounterState {
    #[primary_key]
    pub entity_id: u64,
    pub validation_failure_timestamps: Vec<Timestamp>,
}

#[spacetimedb::table(name = health_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct HealthState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub last_health_decrease_timestamp: Timestamp,
    pub health: f32,
    pub died_timestamp: i32,
}

#[spacetimedb::table(name = resource_health_state, public)]
#[derive(Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct ResourceHealthState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub health: i32,
}

#[spacetimedb::table(name = user_moderation_state, public, index(name = target_entity_id, btree(columns = [target_entity_id])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct UserModerationState {
    #[primary_key]
    pub entity_id: u64, // Unique key for the table row

    pub target_entity_id: u64,     // Which user is this for?
    pub created_by_entity_id: u64, // Who placed this punichment?

    pub user_moderation_policy: UserModerationPolicy, // For (user Id, isBlockedLogin).AnyIsValid) -> do not allow login

    pub created_time: Timestamp, // Consider onlineTimestamp
    pub expiration_time: Timestamp,
    pub duration_ms: u64, // While expiary_time_computed and duration_ms are redundant data in some sense, having them recorded in the table will function as a cache and improve the runtime performance
}

#[spacetimedb::table(name = stamina_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct StaminaState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub last_stamina_decrease_timestamp: Timestamp,
    pub stamina: f32,
}

#[spacetimedb::table(name = teleportation_energy_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct TeleportationEnergyState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub energy: f32,
}

#[spacetimedb::table(name = experience_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct ExperienceState {
    #[primary_key]
    pub entity_id: u64,

    pub experience_stacks: Vec<ExperienceStack>,
}

#[spacetimedb::table(name = partial_experience_state)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PartialExperienceState {
    // This table contains experience fractions (0.0 .. 1.0) for given skills.
    // When calling add_experience_f32, we keep the fraction and possibly add 1 extra experience to the gain
    #[primary_key]
    pub entity_id: u64,

    pub experience_stacks: Vec<ExperienceStackF32>,
}


#[spacetimedb::table(name = active_buff_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct ActiveBuffState {
    #[primary_key]
    pub entity_id: u64,

    pub active_buffs: Vec<ActiveBuff>,
}

#[spacetimedb::table(name = knowledge_achievement_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeAchievementState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_battle_action_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeBattleActionState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_building_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeBuildingState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_cargo_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_on_acquire_callback, achievement)]
pub struct KnowledgeCargoState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_construction_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe)]
pub struct KnowledgeConstructionState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_resource_placement_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe)]
pub struct KnowledgeResourcePlacementState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_craft_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe, achievement)]
pub struct KnowledgeCraftState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_enemy_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeEnemyState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_extract_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe)]
pub struct KnowledgeExtractState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_item_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_on_acquire_callback, achievement)]
pub struct KnowledgeItemState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_lore_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeLoreState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_npc_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeNpcState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_resource_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge, achievement)]
pub struct KnowledgeResourceState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_ruins_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_location)]
pub struct KnowledgeRuinsState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeLocationEntry>,
}

#[spacetimedb::table(name = knowledge_claim_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_entity)]
pub struct KnowledgeClaimState {
    #[primary_key]
    pub entity_id: u64,
    pub entries: Vec<KnowledgeEntityEntry>,
}

#[spacetimedb::table(name = knowledge_secondary_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeSecondaryState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_vault_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeVaultState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_deployable_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge)]
pub struct KnowledgeDeployableState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = knowledge_pillar_shaping_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe)]
pub struct KnowledgePillarShapingState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}


#[spacetimedb::table(name = knowledge_paving_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete, knowledge_recipe)]
pub struct KnowledgePavingState {
    #[primary_key]
    pub entity_id: u64,

    pub entries: Vec<KnowledgeEntry>,
}

#[spacetimedb::table(name = chat_message_state, public,
    index(name = owner_entity_id, btree(columns = [owner_entity_id])),
    index(name = channel_id, btree(columns = [channel_id, timestamp])),
    index(name = target_id, btree(columns = [target_id, timestamp])),
    index(name = just_target_id, btree(columns = [target_id])),
    index(name = channel_and_target_id, btree(columns = [channel_id, target_id, timestamp])),
    //index(name = language_code, btree(columns = [language_code])), //I18N
)]
#[derive(Clone, Debug)]
pub struct ChatMessageState {
    #[primary_key]
    pub entity_id: u64,
    pub username: String,
    pub title_id: i32,
    pub channel_id: i32,    // ChatChannel (only used for regional, not targeted chat messages)
    pub target_id: u64,     // Only used in global targeted chat messages. Either player entity id for dms or chat entity id.
    pub text: String,
    pub timestamp: i32,
    pub owner_entity_id: u64,
    //pub language_code: Option<String>, //I18N
}

#[spacetimedb::table(name = equipment_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct EquipmentState {
    #[primary_key]
    pub entity_id: u64,

    pub equipment_slots: Vec<EquipmentSlot>,
}

#[spacetimedb::table(name = inventory_state, public, 
    index(name = owner_entity_id, btree(columns = [owner_entity_id])),
    index(name = player_owner_entity_id, btree(columns = [player_owner_entity_id])))]
#[derive(Clone, PartialEq, Debug, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct InventoryState {
    #[primary_key]
    pub entity_id: u64,

    pub pockets: Vec<Pocket>,
    pub inventory_index: i32, // Used to distinguish between multiple inventories for the same owner, e.g. player inventory (index 0) and toolbelt (index 1)
    pub cargo_index: i32,
    pub owner_entity_id: u64,
    pub player_owner_entity_id: u64, // Used to enforce player permissions for owners with multiple inventories, e.g. banks
}

#[spacetimedb::table(name = footprint_tile_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct FootprintTileState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    // Note that C-style enums count as size/align of 1,
    // regardless of declared `repr` in Rust.
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub footprint_type: FootprintType,
}

#[spacetimedb::table(name = claim_tile_state, public, index(name = claim_id, btree(columns = [claim_id])))]
#[derive(bitcraft_macro::Operations, Clone, Copy, PartialEq)]
#[operations(delete)]
pub struct ClaimTileState {
    #[primary_key]
    pub entity_id: u64,

    pub claim_id: u64,
}

#[spacetimedb::table(name = pillar_shaping_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct PillarShapingState {
    #[primary_key]
    pub entity_id: u64,
    pub pillar_type_id: i32,
}


#[spacetimedb::table(name = paved_tile_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct PavedTileState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub tile_type_id: i32,
    pub related_entity_id: u64, // optional : when this related entitiy is deleted, delete this paving instance as well
}

#[spacetimedb::table(name = user_state, public,
    index(name = entity_id, btree(columns = [entity_id])))]
#[shared_table] //Owned by region module
#[derive(Clone, Debug)]
pub struct UserState {
    #[unique]
    pub identity: Identity,
    #[primary_key]
    pub entity_id: u64,
    pub can_sign_in: bool,
}

#[spacetimedb::table(name = trade_session_state, public,
    index(name = initiator_entity_id, btree(columns = [initiator_entity_id])),
    index(name = acceptor_entity_id, btree(columns = [acceptor_entity_id])),)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct TradeSessionState {
    #[primary_key]
    pub entity_id: u64,

    pub status: TradeSessionStatus,
    pub initiator_entity_id: u64,
    pub acceptor_entity_id: u64,
    pub initiator_offer: Vec<TradePocket>,
    pub acceptor_offer: Vec<TradePocket>,
    pub updated_at: Timestamp,
    pub resolution_message: String,
}

#[spacetimedb::table(name = character_stats_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct CharacterStatsState {
    #[primary_key]
    pub entity_id: u64,

    pub values: Vec<f32>,
}

#[spacetimedb::table(name = progressive_action_state, public,
    index(name = owner_entity_id, btree(columns = [owner_entity_id])),
    index(name = building_entity_id, btree(columns = [building_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct ProgressiveActionState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub building_entity_id: u64,
    pub function_type: i32,
    pub progress: i32,
    pub recipe_id: i32,
    pub craft_count: i32,
    pub last_crit_outcome: i32,
    pub owner_entity_id: u64,
    pub lock_expiration: Timestamp,
    pub preparation: bool, // Whether this is a start_action or the outcome of the action
}

#[spacetimedb::table(name = terraform_progress_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct TerraformProgressState {
    #[primary_key]
    pub entity_id: u64, // the same entity_id as the building
    pub final_height_target: i16, // the final target to reach (can be +/- 5 away from current terrain height)
    pub next_height_target: i16,  // the next height that will be reached
    pub progress: i32,            // the action progress towards the next_height_target
}

#[spacetimedb::table(name = project_site_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct ProjectSiteState {
    #[primary_key]
    pub entity_id: u64,
    pub construction_recipe_id: i32,
    pub resource_placement_recipe_id: i32,
    pub items: Vec<ItemStack>,
    pub cargos: Vec<ItemStack>,
    pub progress: i32,
    pub last_crit_outcome: i32,
    pub owner_id: u64,
    pub direction: i32,
    pub last_hit_timestamp: Timestamp,
}

#[spacetimedb::table(name = player_state, public)]
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PlayerState {
    pub teleport_location: TeleportLocation,
    #[primary_key]
    pub entity_id: u64,
    pub time_played: i32,
    pub session_start_timestamp: i32,
    pub time_signed_in: i32,
    pub sign_in_timestamp: i32,
    pub signed_in: bool, // Keeping this attribute for optimization even if the value could be found by filtering SignedInPlayerState by entityId
    pub traveler_tasks_expiration: i32,
}

#[spacetimedb::table(name = player_username_state, public)]
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PlayerUsernameState { //Replicated on player region and global module
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub username: String,
}

#[spacetimedb::table(name = player_lowercase_username_state, public)]
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PlayerLowercaseUsernameState { //Replicated on player region and global module
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub username_lowercase: String,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PlayerActionLayer")]
#[repr(u8)]
pub enum PlayerActionLayer {
    Base,
    UpperBody,
}

#[spacetimedb::table(name = player_action_state, public,
    index(name = entity_id, btree(columns = [entity_id])),
    index(name = chunk_index, btree(columns = [chunk_index])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
#[repr(C)]
pub struct PlayerActionState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    #[auto_inc]
    pub auto_id: u64,
    pub chunk_index: u64,
    pub entity_id: u64,
    pub start_time: u64,
    pub duration: u64,
    pub target: Option<u64>,
    pub recipe_id: Option<i32>,
    pub action_type: PlayerActionType,
    pub layer: PlayerActionLayer,
    pub last_action_result: PlayerActionResult,
    pub client_cancel: bool, // don't interrupt the actoin again on the client upon receiving this state change
    // Manual padding to make the struct exactly 72 bytes (see comment in MobileEntityState).
    pub _pad: u32,

}
// Ensure that we don't have hidden padding in the struct
const _: () = assert!(size_of::<PlayerActionState>() == 72);

#[spacetimedb::table(name = enemy_state, public, index(name = herd_entity_id, btree(columns = [herd_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct EnemyState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    // Note that C-style enums count as size/align of 1,
    // regardless of declared `repr` in Rust.
    #[primary_key]
    pub entity_id: u64,
    pub herd_entity_id: u64,
    pub direction: i32,      // unused
    pub status: EnemyStatus, // obsolete
    pub last_ranged_attack_timestamp: Timestamp,
    pub enemy_type: EnemyType,
}

#[spacetimedb::table(name = enemy_scaling_state)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct EnemyScalingState {
    #[primary_key]
    pub entity_id: u64,
    pub enemy_scaling_id: i32,
}

// This is tied to the player's (id and location), but contains the information about the player's deployable
#[spacetimedb::table(name = deployable_collectible_state_v2, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct DeployableCollectibleStateV2 {
    #[primary_key]
    pub deployable_entity_id: u64,
    pub owner_entity_id: u64,
    pub collectible_id: i32,
    pub deployable_desc_id: i32,
    pub location: Option<OffsetCoordinatesSmallMessage>,
    pub auto_follow: bool,
}

// This is tied to the player's (id and location), but contains the information about the player's deployable
#[spacetimedb::table(name = deployable_collectible_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct DeployableCollectibleState { // DEPRECATED
    #[primary_key]
    pub deployable_entity_id: u64,
    pub owner_entity_id: u64,
    pub collectible_id: i32,
    pub deployable_desc_id: i32,
    pub location: Option<OffsetCoordinatesSmallMessage>,
}


#[spacetimedb::table(name = deployable_state, public, 
    index(name = owner_id, btree(columns = [owner_id])), 
    index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct DeployableState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub owner_id: u64,
    pub claim_entity_id: u64,
    pub direction: i32, // for deployables and initial orientation. Not updated in realtime as you move.
    pub deployable_description_id: i32,
    pub nickname: String, //This will be used as tooltip text
    pub hidden: bool,
}

#[spacetimedb::table(name = mounting_state, public, index(name = deployable_entity_id, btree(columns = [deployable_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct MountingState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub deployable_entity_id: u64,
    pub deployable_slot: i32,
}

#[spacetimedb::table(name = npc_state, public, index(name = building_entity_id, btree(columns = [building_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct NpcState {
    #[primary_key]
    pub entity_id: u64,

    pub npc_type: NpcType,
    pub direction: i32,
    pub building_entity_id: u64,
    pub next_action_timestamp: Timestamp,
    pub move_duration: f32,
    pub started_moving: u64,
    pub previous_buildings: Vec<u64>,
    pub traveling: bool,
}

#[spacetimedb::table(name = trade_order_state, public, index(name = shop_entity_id, btree(columns = [shop_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct TradeOrderState {
    #[primary_key]
    pub entity_id: u64,

    pub shop_entity_id: u64, // can be a deployable or a building
    pub remaining_stock: i32,
    pub offer_items: Vec<ItemStack>,
    pub offer_cargo_id: Vec<i32>,   // # MIGRATION # OBSOLETE
    pub required_items: Vec<ItemStack>,
    pub required_cargo_id: Vec<i32>,    // # MIGRATION # OBSOLETE
    pub traveler_trade_order_id: Option<i32>,
}

#[spacetimedb::table(name = claim_recruitment_state, public)]
#[derive(Clone, Debug)]
pub struct ClaimRecruitmentState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub claim_entity_id: u64,
    pub remaining_stock: i32,
    pub required_skill_id: i32,
    pub required_skill_level: i32,
    pub required_approval: bool,
}

// index is added for client as it's in a potential hot loop and is a heavy table
#[spacetimedb::table(name = resource_state, public, index(name = resource_id, btree(columns = [resource_id])))]
#[derive(Default, Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct ResourceState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub resource_id: i32,
    pub direction_index: i32,
}

#[spacetimedb::table(name = building_state, public, 
    index(name = claim_entity_id, btree(columns = [claim_entity_id])), 
    index(name = building_description_id, btree(columns = [building_description_id])))]
#[shared_table]
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
pub struct BuildingState {
    #[primary_key]
    pub entity_id: u64,

    pub claim_entity_id: u64,
    pub direction_index: i32,
    pub building_description_id: i32,
    pub constructed_by_player_entity_id: u64,
}

#[spacetimedb::table(name = building_nickname_state, public)]
#[shared_table] //Owned by region, replicated to global
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
pub struct BuildingNicknameState {
    #[primary_key]
    pub entity_id: u64,
    pub nickname: String,
}

#[spacetimedb::table(name = claim_tech_state, public)]
#[derive(Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct ClaimTechState {
    #[primary_key]
    pub entity_id: u64,
    pub learned: Vec<i32>,
    pub researching: i32,
    pub start_timestamp: Timestamp,
    pub scheduled_id: Option<u64>,
}

#[spacetimedb::table(name = target_state, public, index(name = target_entity_id, btree(columns = [target_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct TargetState {
    #[primary_key]
    pub entity_id: u64,

    pub target_entity_id: u64,
}

#[spacetimedb::table(name = combat_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct CombatState {
    #[primary_key]
    pub entity_id: u64,
    pub last_attacked_timestamp: u64,
    pub last_performed_action_entity_id: u64,
    pub global_cooldown: Option<ActionCooldown>,
}

#[spacetimedb::table(name = threat_state, public, 
    index(name = owner_entity_id, btree(columns = [owner_entity_id])), 
    index(name = target_entity_id, btree(columns = [target_entity_id])))]
#[derive(Clone, Debug)]
pub struct ThreatState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub target_entity_id: u64,
    pub threat: f32,
}

#[spacetimedb::table(name = action_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct ActionState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub action_id: i32,
    pub cooldown: ActionCooldown,
}

#[spacetimedb::table(name = toolbar_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct ToolbarState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub index: u8,
    pub actions: Vec<u64>,
}

#[spacetimedb::table(name = attack_outcome_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct AttackOutcomeState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    pub last_attacked_timestamp: u64, // to catch an update if all the values are the same
    pub damage: i32,
    pub crit_result: bool,
    pub dodge_result: bool,
}

#[spacetimedb::table(name = extract_outcome_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct ExtractOutcomeState {
    #[primary_key]
    pub entity_id: u64,         // set on the player's to optimize due to the large amount of resources
    pub target_entity_id: u64,   // resource's
    pub last_timestamp: Timestamp, // to catch an update if all the values are the same
    pub damage: i32,
}


#[spacetimedb::table(name = targetable_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct TargetableState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
}

#[spacetimedb::table(name = claim_state, public, 
    index(name = owner_player_entity_id, btree(columns = [owner_player_entity_id])),
    index(name = name, btree(columns = [name])), 
    index(name = neutral, btree(columns = [neutral])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[shared_table] //Owned by region, replicated to global module
#[operations(delete)]
pub struct ClaimState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_player_entity_id: u64,
    #[unique]
    pub owner_building_entity_id: u64,
    pub name: String,
    pub neutral: bool,
}

#[spacetimedb::table(name = claim_local_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct ClaimLocalState {
    #[primary_key]
    pub entity_id: u64,
    pub supplies: i32,
    pub building_maintenance: f32,
    pub num_tiles: i32,
    pub num_tile_neighbors: u32,
    pub location: Option<OffsetCoordinatesSmallMessage>,
    pub treasury: u32,
    pub xp_gained_since_last_coin_minting: u32,
    pub supplies_purchase_threshold: u32,
    pub supplies_purchase_price: f32,
    pub building_description_id: i32, //used for trackers since BuildingState is not globally subscribed.
}

#[spacetimedb::table(name = claim_local_supply_security_threshold_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct ClaimLocalSupplySecurityThresholdState {
    #[primary_key]
    pub entity_id: u64,
    pub supply_security_threshold_hours: i32,
}


#[spacetimedb::table(name = claim_member_state, public, 
    index(name = claim_entity_id, btree(columns = [claim_entity_id])),
    index(name = player_entity_id, btree(columns = [player_entity_id])),
    index(name = player_claim, btree(columns = [player_entity_id, claim_entity_id])))]
#[shared_table] //Owned by region, replicated to global module
#[derive(Clone, Debug)]
pub struct ClaimMemberState {
    #[primary_key]
    pub entity_id: u64,
    pub claim_entity_id: u64,
    pub player_entity_id: u64,
    pub user_name: String,
    pub inventory_permission: bool,
    pub build_permission: bool,
    pub officer_permission: bool,
    pub co_owner_permission: bool,
}

#[spacetimedb::table(name = vault_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct VaultState {
    #[primary_key]
    pub entity_id: u64,

    pub collectibles: Vec<VaultCollectible>,
}

#[spacetimedb::table(name = exploration_chunks_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct ExplorationChunksState {
    #[primary_key]
    pub entity_id: u64,

    pub bitmap: Vec<u64>, //Essentially a bitfield. Index=(Z*W+X)/64, bit=(Z*W+X)%64
    pub explored_chunks_count: i32,
}

#[spacetimedb::table(name = loot_chest_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct LootChestState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub building_entity_id: u64,
    pub loot_chest_id: i32,
    pub direction_index: i32,
    pub building_spawn_id: i32,
}

#[derive(Clone)]
#[spacetimedb::table(name = dropped_inventory_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
pub struct DroppedInventoryState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,   // Should be Option<u64> but it seems you can't index an Option<u64> on the client sdk
    pub active_timer_id: u64,
}


#[spacetimedb::table(name = dimension_description_state, public, index(name = dimension_network_entity_id, btree(columns = [dimension_network_entity_id])))]
#[derive(Default, Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct DimensionDescriptionState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    // Note that C-style enums count as size/align of 1,
    // regardless of declared `repr` in Rust.
    #[primary_key]
    pub entity_id: u64,
    pub dimension_network_entity_id: u64,
    pub collapse_timestamp: u64,
    pub interior_instance_id: i32,
    pub dimension_position_large_x: u32, //In large tiles
    pub dimension_position_large_z: u32, //In large tiles
    pub dimension_size_large_x: u32, //In large tiles
    pub dimension_size_large_z: u32, //In large tiles

    #[unique]
    pub dimension_id: u32,
    pub dimension_type: DimensionType,
}

#[spacetimedb::table(name = dimension_network_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct DimensionNetworkState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub building_id: u64,
    pub collapse_respawn_timestamp: u64,

    pub rent_entity_id: u64,
    pub claim_entity_id: u64,

    pub entrance_dimension_id: u32,
    pub is_collapsed: bool,
}

#[spacetimedb::table(name = terrain_chunk_state, public, index(name = dimension, btree(columns = [dimension])))]
#[derive(Default, Clone)]
pub struct TerrainChunkState {
    #[primary_key]
    // chunk_index = (dimension-1)*1000000 + chunk_z * 1000 + chunk_x + 1
    pub chunk_index: u64,

    pub chunk_x: i32,
    pub chunk_z: i32,
    pub dimension: u32,

    pub biomes: Vec<u32>,        // bitfield
    pub biome_density: Vec<u32>, // bitfield
    pub elevations: Vec<i16>,
    pub water_levels: Vec<i16>,
    pub water_body_types: Vec<u8>,
    pub zoning_types: Vec<u8>,
    pub original_elevations: Vec<i16>,
}

#[spacetimedb::table(name = portal_state, public, 
    index(name = destination_dimension, btree(columns = [destination_dimension])),
    index(name = target_building_entity_id, btree(columns = [target_building_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct PortalState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    #[primary_key]
    pub entity_id: u64,

    pub target_building_entity_id: u64,

    pub destination_x: i32,         // cannot name x since we need to join with location table in subscriptions
    pub destination_z: i32,         // cannot name z since we need to join with location table in subscriptions
    pub destination_dimension: u32, // cannot name dimension since we need to join with location table in subscriptions
    pub enabled: bool,
    pub allow_deployables: bool,
}

#[spacetimedb::table(name = interior_collapse_trigger_state, public)]
#[derive(bitcraft_macro::Operations, Clone)]
#[operations(delete)]
pub struct InteriorCollapseTriggerState {
    #[primary_key]
    pub entity_id: u64,

    pub dimension_network_entity_id: u64,
}

#[spacetimedb::table(name = rent_state, public, index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
#[derive(Clone, Debug)]
pub struct RentState {
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub dimension_network_id: u64,
    pub claim_entity_id: u64,
    pub white_list: Vec<u64>,
    pub daily_rent: u32,
    pub paid_rent: u32,
    pub active: bool,
    pub defaulted: bool,
    pub eviction_timestamp: Option<Timestamp>,
}

#[spacetimedb::table(name = satiation_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct SatiationState {
    #[primary_key]
    pub entity_id: u64,
    pub satiation: f32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct KnowledgeEntry {
    pub id: i32,
    pub state: KnowledgeState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct KnowledgeEntityEntry {
    pub entity_id: u64,
    pub state: KnowledgeState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct KnowledgeLocationEntry {
    pub location: OffsetCoordinatesSmallMessage,
    pub state: KnowledgeState,
}

#[derive(Default, Clone, SpacetimeType, Debug)]
pub struct TeleportLocation {
    pub location: OffsetCoordinatesSmallMessage,
    pub location_type: TeleportLocationType,
}

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct ActionCooldown {
    pub timestamp: u64,
    pub cooldown: f32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct VaultCollectible {
    pub id: i32,
    pub activated: bool,
    pub count: i32,
}

#[derive(SpacetimeType, Debug, Default, Copy, Clone)]
pub struct TerrainCell {
    pub x: i32,
    pub z: i32,
    pub dimension: u32,
    pub elevation: i16,
    pub water_level: i16,
    pub biomes: u32,        // bitfield
    pub biome_density: u32, // bitfield
    pub original_elevation: i16,
    pub zoning_type: u8,
    pub water_body_type: u8,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct Threat {
    pub entity_id: u64,
    pub amount: f32,
}

#[spacetimedb::table(name = passive_craft_state, public, 
    index(name = building_entity_id, btree(columns = [building_entity_id])), 
    index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct PassiveCraftState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub recipe_id: i32,
    pub building_entity_id: u64,
    pub timestamp: Timestamp,
    pub status: PassiveCraftStatus,
    pub slot: Option<u32>,
}

#[spacetimedb::table(name = growth_state, public)]
#[derive(bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct GrowthState {
    #[primary_key]
    pub entity_id: u64,
    pub end_timestamp: Timestamp,
    pub growth_recipe_id: i32,
}

#[spacetimedb::table(name = player_prefs_state, public)]
#[derive(bitcraft_macro::Operations, Debug, Clone)]
#[operations(delete)]
pub struct PlayerPrefsState {
    #[primary_key]
    pub entity_id: u64,

    // This might be a place-holder that will be removed once we can place a collectible on the action bar.
    pub default_deployable_collectible_id: i32,
}

#[spacetimedb::table(name = alert_state, public, index(name = player_entity_id, btree(columns = [player_entity_id])))]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct AlertState {
    // Sort fields in order of decreasing size/alignment
    // to take advantage of a serialization fast-path in SpacetimeDB.
    // Note that C-style enums count as size/align of 1,
    // regardless of declared `repr` in Rust.
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,
    pub target_entity_id: u64,
    pub end_timestamp: Timestamp,
    pub alert_type: AlertType,
}

#[spacetimedb::table(name = onboarding_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct OnboardingState {
    #[primary_key]
    pub entity_id: u64,
    pub completed_states: Vec<u16>,
    pub current_quests: Vec<u16>,
    pub completed_quests: Vec<u16>,
}

#[spacetimedb::table(name = signed_in_player_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct SignedInPlayerState {
    #[primary_key]
    pub entity_id: u64,
}

#[spacetimedb::table(name = unclaimed_shards_state)]
#[derive(Clone, Debug)]
pub struct UnclaimedShardsState {
    #[primary_key]
    pub identity: Identity,
    pub shards: u32,
}

#[spacetimedb::table(name = unclaimed_collectibles_state)]
#[derive(Clone, Debug)]
pub struct UnclaimedCollectiblesState {
    #[primary_key]
    pub identity: Identity,
    pub collectibles: Vec<i32>,
}

#[spacetimedb::table(name = player_timestamp_state)]
#[derive(Clone)]
#[repr(C)]
pub struct PlayerTimestampState {
    #[primary_key]
    pub entity_id: u64,
    pub timestamp: Timestamp,
}
// Ensure that we don't have hidden padding in the struct
const _: () = assert!(size_of::<PlayerTimestampState>() == 16);

#[spacetimedb::table(name = auto_claim_state)]
pub struct AutoClaimState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
}

#[spacetimedb::table(name = herd_state, public, index(name = enemy_ai_params_desc_id, btree(columns = [enemy_ai_params_desc_id])))]
#[derive(Clone, Debug)]
pub struct HerdState {
    #[primary_key]
    pub entity_id: u64,
    pub enemy_ai_params_desc_id: i32,
    pub current_population: i32,
    pub ignore_eagerness: bool,
    pub population_variance: Vec<f32>,
    #[default(0u64)]
    pub crumb_trail_entity_id: u64,     // for prize herds
}

// This table is required because you can have multiple herds (1 for each type of enemy) attached to a single entity and herds need an unique entity_id.
#[spacetimedb::table(name = attached_herds_state, public)]
#[derive(Debug, Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct AttachedHerdsState {
    #[primary_key]
    pub entity_id: u64,
    pub herds_entity_ids: Vec<u64>,
}

// This table is required because you can have multiple herds (1 for each type of enemy) attached to a single entity and herds need an unique entity_id.
#[spacetimedb::table(name = enemy_mob_monitor_state, public)]
#[derive(Debug, Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct EnemyMobMonitorState {
    #[primary_key]
    pub entity_id: u64,
    pub enemy_type: EnemyType,
    pub herd_entity_id: u64,
    pub herd_location: OffsetCoordinatesSmallMessage,
}

#[spacetimedb::table(name = starving_player_state)]
pub struct StarvingPlayerState {
    #[primary_key]
    pub entity_id: u64,
}

#[spacetimedb::table(name = barter_stall_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct BarterStallState {
    #[primary_key]
    pub entity_id: u64,
    pub market_mode_enabled: bool, //This has been discontinued
}

#[spacetimedb::table(name = player_note_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct PlayerNoteState {
    #[primary_key]
    pub entity_id: u64,
    pub text: String,
}

#[spacetimedb::table(name = light_source_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct LightSourceState {
    #[primary_key]
    pub entity_id: u64,
    pub radius: f32,
}

#[spacetimedb::table(name = a_i_debug_state, public)]
pub struct AIDebugState {
    #[primary_key]
    pub entity_id: u64,
    pub target_entity_id: u64,
    pub current_destination: OffsetCoordinatesFloat,
    pub current_position: OffsetCoordinatesFloat,
    pub target_position: OffsetCoordinatesFloat,
    pub dp: f32,
}

// Sometimes (like in onboarding) we need to search the whole world for a particular building and we call a reducer like search_for_closest_building.
// A reducer like that will output the result into this table, keyed by the calling player's entity id.
#[spacetimedb::table(name = global_search_state, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct GlobalSearchState {
    #[primary_key]
    pub entity_id: u64,
    pub found_entity_id: u64,
    pub found_entity_name: String,
    pub x: i32,
    pub z: i32,
    pub timestamp: Timestamp,
}

// If a type is not included in a table or a reducer, it is not generated on the client.
// Enums can't be indexed as of version 1.0.0, and need to be represented as a native type.
// Therefore, until this feature is supported once more, we have to add those types in a table so they get auto-generated.
#[spacetimedb::table(name = the_great_placeholder_table, public)]
pub struct TheGreatPlaceHolderTable {
    #[primary_key]
    pub placeholder_id: u64,
    pub placeholder_skill_type: SkillType,
    pub placeholder_buff_category: BuffCategory,
    pub placeholder: ProgressiveActionStatus,
    pub placeholder_water_body_type: SurfaceType,
    pub chat_channel: ChatChannel,
    pub permission: Permission,
    pub permission_group: PermissionGroup,
    #[default(AbilityTypeEnum::_Unsupported)]
    pub ability_type: AbilityTypeEnum,
}

#[spacetimedb::table(name = traveler_task_state, public, 
    index(name = player_entity_id, btree(columns = [player_entity_id])),
    index(name = per_player_and_traveler_id, btree(columns = [player_entity_id, traveler_id])))]
#[derive(Clone, Debug)]
pub struct TravelerTaskState {
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,
    pub traveler_id: i32,
    pub task_id: i32,
    pub completed: bool,
}

#[spacetimedb::table(name = sell_order_state, public, 
    index(name = item_for_claim, btree(columns = [item_id, item_type, claim_entity_id])),
    index(name = claim_entity_id, btree(columns = [claim_entity_id])),
    index(name = owner_for_claim, btree(columns = [owner_entity_id, claim_entity_id])))]
#[spacetimedb::table(name = buy_order_state, public,
    index(name = item_for_claim, btree(columns = [item_id, item_type, claim_entity_id])),
    index(name = claim_entity_id, btree(columns = [claim_entity_id])),
    index(name = owner_for_claim, btree(columns = [owner_entity_id, claim_entity_id])))]
#[derive(Clone, Debug)]
pub struct AuctionListingState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub claim_entity_id: u64,
    pub item_id: i32,
    pub item_type: i32,
    pub price_threshold: i32,
    pub quantity: i32,
    pub timestamp: Timestamp,
    pub stored_coins: i32,
}

#[spacetimedb::table(name = closed_listing_state, public, 
    index(name = owner_for_claim, btree(columns = [owner_entity_id, claim_entity_id])),
    index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
pub struct ClosedListingState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,
    pub claim_entity_id: u64,
    pub item_stack: ItemStack,
    pub timestamp: Timestamp,
}

#[spacetimedb::table(name = distant_visible_entity, public)]
#[derive(Clone, bitcraft_macro::Operations, Debug)]
#[operations(delete)]
pub struct DistantVisibleEntity {
    #[primary_key]
    pub entity_id: u64,
    pub chunk_index: u64, //Needed because we can't double join or do a where on the left table when joining
}

// Keep all players affected by long term rez sickness to improve player_move
#[spacetimedb::table(name = rez_sick_long_term_state)]
#[derive(Clone, bitcraft_macro::Operations)]
#[operations(delete)]
pub struct RezSickLongTermState {
    #[primary_key]
    pub entity_id: u64,
}

#[spacetimedb::table(name = duel_state, public)]
pub struct DuelState {
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub initiator_entity_id: u64,
    #[unique]
    pub acceptor_entity_id: u64,
    pub victor: Option<u64>,
    pub player_entity_ids: Vec<u64>,    // internal use, contains initiator & acceptor
    pub out_of_range_timestamps: Vec<Option<Timestamp>>,
}

// Keep all players affected by long term rez sickness to improve player_move
#[derive(Clone, Copy, Debug)]
#[spacetimedb::table(name = permission_state, public,
    index(name = ordained_entity_id, btree(columns = [ordained_entity_id])),
    index(name = allowed_entity_id, btree(columns = [allowed_entity_id])),
    index(name = ordained_and_allowed_entity_id, btree(columns = [ordained_entity_id, allowed_entity_id])))]
pub struct PermissionState {
    #[primary_key]
    pub entity_id: u64,
    pub ordained_entity_id: u64,    // Claim, Interior, etc. that is effected by permissions
    pub allowed_entity_id: u64,       // Player, Claim, Empire, Interior, etc.
    pub group: i32,                 // Preset for faster sorting
    pub rank: i32,                  // Permission Enum value
}

#[derive(Clone, Debug)]
#[shared_table] //Owned by regions, replicated to global module
#[spacetimedb::table(name = player_housing_state, public, index(name = entrance_building_entity_id, btree(columns = [entrance_building_entity_id])))]
pub struct PlayerHousingState {
    #[primary_key]
    pub entity_id: u64,
    pub entrance_building_entity_id: u64,
    #[unique]
    pub network_entity_id: u64,
    pub exit_portal_entity_id: u64, // from inside the housing dimension
    pub rank: i32,
    pub locked_until: Timestamp,   // a housing needs to be empty to lock down for cross-region transport.
    pub is_empty: bool,             // updated whenever the player leaves his housing
    pub region_index: u8,
}

#[derive(Clone, Debug)]
#[spacetimedb::table(name = player_housing_moving_cost_state, public)]
pub struct PlayerHousingMovingCostState {
    #[primary_key]
    pub entity_id: u64,
    pub moving_time_cost_minutes: i32,
}

#[spacetimedb::table(name = lost_items_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone)]
pub struct LostItemsState {
    #[primary_key]
    pub inventory_entity_id: u64,   
    pub owner_entity_id: u64,
    pub location: OffsetCoordinatesSmallMessage,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "NotificationSeverity")]
#[repr(u8)]
pub enum NotificationSeverity {
    Info, // General information, nothing went wrong, e.g. You've explored x% of the world.
    ActionDenied, // Not an error, but an action has been denied, e.g. Not enough stamina.
    ReducerError, // Something went very wrong in the reducer, e.g. This player doesn't have a HealthState.
}

#[event_table(name = player_notification_event)]
pub struct PlayerNotificationEvent {
    pub player_entity_id: u64,
    pub message: String,
    pub severity: NotificationSeverity,
}

#[spacetimedb::table(name = user_previous_region_state)]
#[derive(Clone, Debug)]
pub struct UserPreviousRegionState {
    #[primary_key]
    pub identity: Identity,
    pub previous_region_location: FloatHexTileMessage,
    pub with_vehicle: bool,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,
}

#[spacetimedb::table(name = contribution_state, public, 
    index(name = player_enemy_entity_id, btree(columns = [player_entity_id, enemy_entity_id])),
    index(name = enemy_entity_id, btree(columns = [enemy_entity_id])))]
pub struct ContributionState {
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,
    pub enemy_entity_id: u64,
    pub contribution: f32,
}


#[spacetimedb::table(name = player_report_state)]
#[shared_table] //Created by regions and collected on global module
#[custom_inter_module_insert]
#[derive(Clone, Debug)]
pub struct PlayerReportState {
    #[primary_key]
    pub entity_id: u64,
    pub reporter_entity_id: u64,
    pub reported_player_entity_id: u64,
    pub reported_player_username: String,
    pub report_type: String,
    pub report_message: String,
    pub reported_chat_message: Option<ChatMessageState>,
    pub chat_channel_context: Option<Vec<ChatMessageState>>,
    pub chat_user_context: Option<Vec<ChatMessageState>>,
    pub actioned: bool,
}

#[spacetimedb::table(name = player_report_state_timestamp)]
#[derive(Clone, Debug)]
pub struct PlayerReportStateTimestamp {
    #[primary_key]
    pub entity_id: u64, //same as player_report_state entity_id
    pub timestamp: i32,
}


#[spacetimedb::table(name = combat_dimension_state, public)]
pub struct CombatDimensionState {
    #[primary_key]
    pub dimension_id: u32,
}


#[spacetimedb::table(name = moderation_action_log_entry,
    index(name = report_entity_id, btree(columns = [report_entity_id])),
    index(name = reported_player_entity_id, btree(columns = [reported_player_entity_id])))]
#[derive(Clone, Debug)]
pub struct ModerationActionLogEntry {
    #[primary_key]
    pub entity_id: u64,
    pub report_entity_id: u64,
    pub reported_player_entity_id: u64,
    pub admin_name: String,
    pub reported_player_username: String,
    pub action_type: String,
    pub moderation_notice: String,
    pub details: String,
    pub timestamp: i32
}

#[spacetimedb::table(name = public_progressive_action_state, public, index(name = building_entity_id, btree(columns = [building_entity_id])))]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct PublicProgressiveActionState {
    #[primary_key]
    pub entity_id: u64,
    pub building_entity_id: u64,
    pub owner_entity_id: u64,
}

#[spacetimedb::table(name = dungeon_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct DungeonState {
    #[primary_key]
    pub entity_id: u64, //Same as building entity id
    pub location: OffsetCoordinatesSmallMessage, //Makes things easier on client
}

#[spacetimedb::table(name = interior_player_count_state, public)]
#[derive(bitcraft_macro::Operations, Clone, Debug)]
#[operations(delete)]
pub struct InteriorPlayerCountState {
    #[primary_key]
    pub entity_id: u64, //Same as building entity id
    #[unique]
    pub dimension_network_entity_id: u64,
    pub player_count: u32,
}

#[derive(spacetimedb::SpacetimeType, Clone, PartialEq, Debug)]
pub enum ActionLogData {
    Reserved(ActionLogDataSpaceAllocator),//TODOn't: Make ActionLogData bigger so that we can add more data types without migration support
    WithdrawItem(ItemStack),
    DepositItem(ItemStack),
}

#[derive(spacetimedb::SpacetimeType, Clone, PartialEq, Debug)]
pub struct ActionLogDataSpaceAllocator {
    //TODOn't: Make ActionLogData bigger so that we can add more data types without migration support
    pub item1: ItemStack,
    pub item2: ItemStack,
    pub val1: u64,
    pub val2: u64,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
pub enum ActionLogSubjectType {
    Player = 0,
}

#[spacetimedb::table(name = storage_log_state, public, 
    index(name = object_entity_id, btree(columns = [object_entity_id])),
    index(name = days_since_epoch, btree(columns = [days_since_epoch])))]
#[derive(Clone, Debug)]
pub struct ActionLogState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub object_entity_id: u64, //Who owns this log entry. Can be building, empire, claim, etc
    pub subject_entity_id: u64, //Who performed this action. Can be player, empire, etc
    pub subject_name: String,
    pub subject_type: ActionLogSubjectType,
    pub data: ActionLogData,
    pub timestamp: Timestamp,
    pub days_since_epoch: i32
}

#[spacetimedb::table(name = waystone_state, public, 
    index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
#[derive(Clone, Debug)]
pub struct WaystoneState {
    #[primary_key]
    pub building_entity_id: u64,
    pub claim_entity_id: u64,
    pub coordinates: SmallHexTileMessage,
}

#[spacetimedb::table(name = bank_state, public, 
    index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
#[derive(Clone, Debug)]
pub struct BankState {
    #[primary_key]
    pub building_entity_id: u64,
    pub claim_entity_id: u64,
    pub coordinates: SmallHexTileMessage,

}

#[spacetimedb::table(name = marketplace_state, public,
    index(name = claim_entity_id, btree(columns = [claim_entity_id])))]
#[derive(Clone, Debug)]
pub struct MarketplaceState {
    #[primary_key]
    pub building_entity_id: u64,
    pub claim_entity_id: u64,
    pub coordinates: SmallHexTileMessage,
}

#[spacetimedb::table(name = player_settings_state, public)]
#[derive(Clone, Debug)]
pub struct PlayerSettingsState { // DEPRECATED
    #[primary_key]
    pub entity_id: u64,
    pub fill_deployable_inventory_first: bool,
}

#[spacetimedb::table(name = player_settings_state_v2, public)]
#[derive(Clone, Debug)]
pub struct PlayerSettingsStateV2 {
    #[primary_key]
    pub entity_id: u64,
    pub fill_player_inventory: bool,
    pub fill_deployable_inventory_first: bool,
}

// Keep in sync with AbilityTypeEnum
#[derive(SpacetimeType, Debug, Copy, Clone, Eq, PartialEq)]
pub enum AbilityType {
    _Unsupported(u128),         // Padding for future types
    Eat(i32),                   // Item Id (e.g. Eat Apple)
    CombatAction(i32),          // Action Id
    AutoAttack,
    Custom(i32),                // AbilityCustomDesc Id // default ability with default settings
    Prospecting(i32),           // Prospecting ID
    Equip(i32),                 // Item Id
    DeployableDeploy(i32),      // Collectible ID - likely unused, replaced by DeployableToggle
    AddToToolbelt(i32),         // Item Id
    DeployableToggle(i32),      // Collectible ID - will call either Store or Deploy based on the collectible state
}

// Keep in sync with AbilityType
#[derive(SpacetimeType, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(i32)]
pub enum AbilityTypeEnum {
    _Unsupported,
    Eat,
    CombatAction,
    AutoAttack,
    SelfBuff,
    Custom,
    Prospecting,
    Equip,
    DeployableDeploy,       // - likely unused, replaced by DeployableToggle
    AddToToolbelt,
    DeployableToggle,
}

#[spacetimedb::table(name = action_bar_state, public,
     index(name = player_entity_id, btree(columns = [player_entity_id])),
     index(name = by_player_slot, btree(columns = [player_entity_id, action_bar_index, local_ability_index])))]
#[derive(Clone, Debug)]
pub struct ActionBarState {
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,          // for subscription
    pub action_bar_index: u8,           // action bar #
    pub local_ability_index: u8,        // local position in the action bar
    pub ability_entity_id: u64,
}

#[spacetimedb::table(name = ability_state, public, index(name = owner_entity_id, btree(columns = [owner_entity_id])))]
#[derive(Clone, Debug)]
pub struct AbilityState {
    #[primary_key]
    pub entity_id: u64,
    pub owner_entity_id: u64,          // for subscription
    pub ability: AbilityType,
    pub cooldown: ActionCooldown,      
}

#[spacetimedb::table(name = prospecting_state, public, index(name = crumb_trail_entity_id, btree(columns = [crumb_trail_entity_id])))]
#[derive(Clone, Debug)]
pub struct ProspectingState {
    #[primary_key]
    pub entity_id: u64,                 // player
    pub prospecting_id: i32,            // in case the player wants to start a different type of prospecting
    pub crumb_trail_entity_id: u64,
    pub completed_steps: i32,
    pub ongoing_step: i32,
    pub total_steps: i32,
    pub next_crumb_angle: Vec<f32>,
    pub last_prospection_timestamp: Timestamp,    // force an update callback on consecutive misses
    pub contribution: i32,
}

#[spacetimedb::table(name = crumb_trail_state)]
#[derive(Clone, Debug)]
pub struct CrumbTrailState {
    #[primary_key]
    pub entity_id: u64,
    pub original_location: OffsetCoordinatesSmallMessage,   // Used until the first crumb is found
    pub crumb_locations: Vec<OffsetCoordinatesSmallMessage>,
    pub crumb_radiuses: Vec<i32>,                           // In small tiles
    pub prize_location: OffsetCoordinatesSmallMessage,
    pub active_step: i32,
    pub prize_entity_ids: Vec<u64>,
    pub join_radius: i32, // Copy from static data for faster parsing
    pub clean_up_counter: i32, // delete trail when it reaches 3
}

#[spacetimedb::table(name = crumb_trail_contribution_lock_state, public, index(name = crumb_trail_entity_id, btree(columns = [crumb_trail_entity_id])))]
#[derive(Clone, Debug)]
pub struct CrumbTrailContributionLockState {
    #[primary_key]
    pub entity_id: u64,
    pub crumb_trail_entity_id: u64,
}

// A bit sad, but this is needed to have a different error message to act as a memory that the player is out of contribution
#[spacetimedb::table(name = crumb_trail_contribution_spent_state, public, 
    index(name = crumb_trail_entity_id, btree(columns = [crumb_trail_entity_id])),
    index(name = player_and_crumb_entity_id, btree(columns = [player_entity_id, crumb_trail_entity_id])))]
#[derive(Clone, Debug)]
pub struct CrumbTrailContributionSpentState {
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,
    pub crumb_trail_entity_id: u64,
}

#[spacetimedb::table(name = quest_chain_state, public,
    index(name = player_entity_id, btree(columns = [player_entity_id])))]
#[derive(Clone, Debug)]
pub struct QuestChainState {
    #[primary_key]
    pub entity_id: u64,
    pub player_entity_id: u64,
    pub quest_chain_desc_id: i32,
    pub stage_id: i32,
    pub is_active: bool,
    pub completed: bool,
    pub stage_rewards_awarded: Vec<i32>,
}
