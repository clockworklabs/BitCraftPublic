use bitcraft_macro::shared_table;

use crate::messages::game_util::PocketKey;

use super::util::OffsetCoordinatesSmallMessage;

#[derive(spacetimedb::SpacetimeType, Clone, Copy, PartialEq, Debug)]
#[sats(name = "EmpirePermission")]
#[repr(i32)]
pub enum EmpirePermission {
    SupplyNode = 0,
    CollectHexiteCapsule = 1,
    BuildWatchtower = 2,
    InviteSettlementToEmpire = 3, // DAB Note: Remove post alpha-2
    FlagWatchtowerToSiege = 4,
    AproveEmpireSubmissions = 5,
    PromoteLesserRanks = 6,
    MarkAreaForExpansion = 7,
    CraftHexiteCapsule = 8,
    Count,
    // CollectSiegeSupplies = 9,
}

#[spacetimedb::table(name = empire_state, public)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireState {
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub capital_building_entity_id: u64,
    #[unique]
    pub name: String,
    pub shard_treasury: u32,
    pub nobility_threshold: i32,
    pub num_claims: i32,
    pub location: OffsetCoordinatesSmallMessage,
}

#[spacetimedb::table(name = empire_node_state, public, 
    index(name = empire_entity_id, btree(columns = [empire_entity_id])),
    index(name = active, btree(columns = [active])),
    index(name = chunk_index, btree(columns = [chunk_index])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireNodeState {
    #[primary_key]
    pub entity_id: u64, // Same as ClaimTotem or WatchTower
    pub empire_entity_id: u64,
    pub chunk_index: u64,
    pub energy: i32,
    pub active: bool,
    pub upkeep: i32,
    pub location: OffsetCoordinatesSmallMessage, // For map purpose
}

#[spacetimedb::table(name = empire_settlement_state, public, 
    index(name = empire_entity_id, btree(columns = [empire_entity_id])),
    index(name = claim_entity_id, btree(columns = [claim_entity_id])),
    index(name = chunk_index, btree(columns = [chunk_index])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireSettlementState {
    #[primary_key]
    pub building_entity_id: u64, // Same as ClaimTotem or WatchTower
    #[unique]
    pub claim_entity_id: u64,
    pub empire_entity_id: u64,
    pub chunk_index: u64,
    pub can_house_empire_storehouse: bool, // not checked for now, need design rules
    pub members_donations: u32,
    pub location: OffsetCoordinatesSmallMessage, // For map purpose
}

#[spacetimedb::table(name = empire_chunk_state, public)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireChunkState {
    #[primary_key]
    pub chunk_index: u64, // Same as TerrainChunkState
    pub empire_entity_id: Vec<u64>,
}

#[spacetimedb::table(name = empire_expansion_state, public)]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireExpansionState {
    //DAB Note: Right now we cannot subscribe ONLY to your empire's expansions, which means that we can fairly-easily see where ALL empires plan to expand (which isn't good for a competitive feature).
    //We can change this struct to be `entity_id: u64, chunk_index: u64, empire_entity_id: u64` to avoid that issue. It will also help optimize some code and reduce how much data we send over the network
    #[primary_key]
    pub chunk_index: u64, // Same as TerrainChunkState
    pub empire_entity_id: Vec<u64>,
}

#[spacetimedb::table(name = empire_player_data_state, public, index(name = empire_entity_id, btree(columns = [empire_entity_id])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpirePlayerDataState {
    #[primary_key]
    pub entity_id: u64, // Player's. By design, a player can only be citizen of one empire
    pub empire_entity_id: u64,
    pub rank: u8,
    pub donated_shards: u32,
    pub noble: Option<Timestamp>, // A player can be a noble without enough donated_shards if that threshold was increased after his promotion
}

#[spacetimedb::table(name = empire_rank_state, public, 
    index(name = empire_entity_id, btree(columns = [empire_entity_id])),
    index(name = empire_rank, btree(columns = [empire_entity_id, rank])),
    index(name = rank, btree(columns = [rank])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireRankState {
    #[primary_key]
    pub entity_id: u64,
    pub empire_entity_id: u64,
    pub rank: u8,
    pub title: String,
    pub permissions: Vec<bool>, //Based on EmpirePermission enum //DAB Note: these can be packed into a u8 (STDB has significant overhead for serializing vecs)
}

#[spacetimedb::table(name = empire_node_siege_state, public,
    index(name = empire_entity_id, btree(columns = [empire_entity_id])),
    index(name = active, btree(columns = [active])),
    index(name = building_entity_id, btree(columns = [building_entity_id])))]
#[shared_table] //Owned by global module, replicated to regions
#[derive(Clone, Debug)]
pub struct EmpireNodeSiegeState {
    #[primary_key]
    pub entity_id: u64,
    pub building_entity_id: u64,
    pub empire_entity_id: u64,
    pub energy: i32,
    pub active: bool, // for faster filtering during siege tick
    pub start_timestamp: Option<Timestamp>,
}

// === Action Requests ========================================================================

#[derive(SpacetimeType)]
pub struct EmpireResupplyNodeRequest {
    pub building_entity_id: u64,
    pub from_pocket: PocketKey,
}

#[derive(SpacetimeType)]
pub struct EmpireCollectHexiteCapsuleRequest {
    pub building_entity_id: u64,
    pub player_entity_id: u64,
}

#[derive(SpacetimeType)]
pub struct EmpireAddSiegeSuppliesRequest {
    pub building_entity_id: u64,
    pub proxy_empire_entity_id: Option<u64>, // if not aligned with either empires pertaking in the siege, player has to choose which empire to favor
}

#[derive(SpacetimeType)]
pub struct EmpireStartSiegeRequest {
    pub building_entity_id: u64,
    pub coord: OffsetCoordinatesSmallMessage,
    pub direction: i32,
}

#[derive(SpacetimeType)]
pub struct EmpireQueueSuppliesRequest {
    pub building_entity_id: u64,
}