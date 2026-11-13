use bitcraft_macro::shared_table;
use spacetimedb::Timestamp;

#[spacetimedb::table(accessor = world_region_state, public)]
#[derive(Debug)]
pub struct WorldRegionState {
    #[primary_key]
    pub id: u8, // there should always be a single row in this table, we can set this id to 0
    pub region_min_chunk_x: u16,   // bottom left corner x in chunk coordinates
    pub region_min_chunk_z: u16,   // bottom left corner z in chunk coordinates
    pub region_width_chunks: u16,  // formerly in GlobalsAppeared
    pub region_height_chunks: u16, // formerly in GlobalsAppeared
    pub region_index: u8,
    pub region_count: u8,
    pub region_count_sqrt: u8,
}

#[spacetimedb::table(accessor = world_region_name_state, public)]
#[derive(Debug)]
pub struct WorldRegionNameState {
    #[primary_key]
    pub id: u16, // there should always be a single row in this table, we can set this id to 0
    pub player_facing_name: String,
    pub module_name_prefix: String, // e.g. "bitcraft_region_"
}

#[spacetimedb::table(accessor = globals, public)]
pub struct Globals {
    #[primary_key]
    pub version: i32,

    pub entity_pk_counter: u64,
    pub dimension_counter: u32,
    pub region_index: u8,
}

#[spacetimedb::table(accessor = admin_broadcast, public)]
pub struct AdminBroadcast {
    #[primary_key]
    pub version: i32,
    pub title: String,
    pub message: String,
    pub sign_out: bool,
    pub timestamp: Timestamp,
}

#[spacetimedb::table(accessor = config)]
#[derive(Clone)]
pub struct Config {
    #[primary_key]
    pub version: i32,
    pub env: String,
    pub agents_enabled: bool,
}

/// To efficiently implement resource respawning,
/// maintain in the database a map from resource type to number of deposits present in the world.
///
/// When consuming a resource deposit, decrement the count for its resource type.
///
/// In the resources regen loop, for each type of resource,
/// compute the difference between the count and the desired number of deposits,
/// spawn resources equal to the difference, and set the count to the desired number.
#[spacetimedb::table(accessor = resource_count)]
pub struct ResourceCount {
    #[primary_key]
    /// The id of the type of resource deposit to which this counter refers.
    pub resource_id: i32,

    /// The number of deposits of this type currently in the world.
    pub num_in_world: i32,
}

#[spacetimedb::table(accessor = region_connection_info, public)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct RegionConnectionInfo {
    #[primary_key]
    pub id: u8,
    pub host: String,
    pub module: String,
}

#[spacetimedb::table(accessor = region_control_info, public)]
#[shared_table] //Owned by regions, replicated to global
#[derive(Clone, Debug)]
pub struct RegionControlInfo {
    //Created after terrain is uploaded. `allow_players` and `allow_player_spawns` default to `true` in dev and `false` otherwise
    #[primary_key]
    pub region_id: u8,
    pub initialized: bool, //Has uploaded terrain, allows GMs to log in
    pub allow_players: bool,
    pub allow_player_spawns: bool,
}

#[spacetimedb::table(accessor = region_population_info, public)]
#[shared_table] //Owned by regions, replicated to global
#[derive(Clone, Debug)]
pub struct RegionPopulationInfo {
    #[primary_key]
    pub region_id: u8,
    pub signed_in_players: u32,
    pub players_in_queue: u32,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "HubItemType")]
#[repr(i32)]
pub enum HubItemType {
    HexiteShards,
    Collectible,
    PremiumItem,
}

#[spacetimedb::table(accessor = region_sign_in_parameters, public)]
#[shared_table] //Owned by global, replicated to regions
#[derive(Clone, Debug)]
pub struct RegionSignInParameters {
    #[primary_key]
    pub region_id: u8,
    pub is_signing_in_blocked: bool,
    pub max_signed_in_players: u64,
    pub max_queue_length: u64,
    pub queue_length_tolerance: u32,
    pub grace_period_seconds: u64,
}

#[spacetimedb::table(accessor = region_exploration_info, public)]
#[shared_table] //Owned by global, replicated to regions
#[derive(Clone, Debug)]
pub struct RegionExplorationInfo {
    #[primary_key]
    pub region_id: u8,
    pub counts_toward_achievements: bool,
}

#[spacetimedb::table(accessor = gated_features)]
pub struct GatedFeature {
    #[primary_key]
    pub feature: String,
}

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "PremiumServiceType")]
#[repr(i32)]
pub enum PremiumServiceType {
    CharacterRename,
}

impl WorldRegionState {
    pub fn world_width_chunks(&self) -> i32 {
        self.region_width_chunks as i32 * self.region_count_sqrt as i32
    }

    pub fn world_height_chunks(&self) -> i32 {
        self.region_height_chunks as i32 * self.region_count_sqrt as i32
    }

    pub fn world_chunk_count(&self) -> i32 {
        self.region_width_chunks as i32 * self.region_height_chunks as i32 * self.region_count as i32
    }
}
