use spacetimedb::SpacetimeType;
use spacetimedb::Timestamp;

use super::static_data::EmpireNotificationType;

// === Tables ========================================================================

#[spacetimedb::table(accessor = empire_emblem_state, public)]
#[derive(Clone, Debug)]
pub struct EmpireEmblemState {
    #[primary_key]
    pub entity_id: u64,
    pub icon_id: i32,
    pub shape_id: i32,
    pub color1_id: i32,
    pub color2_id: i32,
}

#[spacetimedb::table(accessor = empire_directive_state, public)]
#[derive(Clone, Debug)]
pub struct EmpireDirectiveState {
    #[primary_key]
    pub entity_id: u64,
    pub directive_message: String,
    pub directive_message_timestamp: Option<Timestamp>,
}

#[spacetimedb::table(accessor = empire_player_log_state, public, index(accessor = empire_entity_id, btree(columns = [empire_entity_id])))]
#[derive(Clone, Debug)]
pub struct EmpirePlayerLogState {
    #[primary_key]
    pub entity_id: u64, // Player's. By design, a player can only be citizen of one empire
    pub empire_entity_id: u64, // Duplicate information but useful for broadcasting and accessing empire notifications
    pub last_viewed: u64,
}

#[spacetimedb::table(accessor = empire_log_state)]
#[derive(Clone, Debug)]
pub struct EmpireLogState {
    #[primary_key]
    pub entity_id: u64, // EmpireState's.
    pub last_posted: u64,
}

#[spacetimedb::table(accessor = empire_foundry_state, public, index(accessor = empire_entity_id, btree(columns = [empire_entity_id])))]
#[derive(Clone, Debug)]
pub struct EmpireFoundryState {
    #[primary_key]
    pub entity_id: u64,
    pub empire_entity_id: u64,
    pub hexite_capsules: i32,
    pub queued: i32,
    pub started: Timestamp,
}

#[spacetimedb::table(accessor = empire_notification_state, public, index(accessor = empire_entity_id, btree(columns = [empire_entity_id])))]
#[derive(Clone, Debug)]
pub struct EmpireNotificationState {
    #[primary_key]
    pub entity_id: u64,
    pub empire_entity_id: u64,
    pub notification_type: EmpireNotificationType, // for client ui (sprites, format, etc.)
    pub text_replacement: Vec<String>,
    pub timestamp: i32, // DAB Note: for future cleanup (eg agent that cleans up logs older than 1 week every day at midnight). Possibly to display on the client too if required.
}

#[spacetimedb::table(accessor = empire_siege_engine_state, public)]
#[derive(Clone, Debug)]
pub struct EmpireSiegeEngineState {
    #[primary_key]
    pub entity_id: u64,
    #[unique]
    pub building_entity_id: u64,
}

#[spacetimedb::table(accessor = empire_blacklist_state, public,
    index(accessor = empire_entity_id, btree(columns = [empire_entity_id])),
    index(accessor = empire_player, btree(columns = [empire_entity_id, player_entity_id])))]
#[derive(Clone, Debug)]
pub struct EmpireBlacklistState {
    #[primary_key]
    #[auto_inc]
    pub entity_id: u64,
    pub empire_entity_id: u64,
    pub player_entity_id: u64,
}

// === Action Requests ========================================================================

#[derive(SpacetimeType)]
pub struct EmpireFormRequest {
    pub building_entity_id: u64,
    pub empire_name: String,
    pub icon_id: i32,
    pub shape_id: i32,
    pub color1_id: i32,
    pub color2_id: i32,
}

#[derive(SpacetimeType)]
pub struct EmpireSetPlayerRankRequest {
    pub empire_entity_id: u64,
    pub player_entity_id: u64,
    pub rank: u8,
}

#[derive(SpacetimeType)]
pub struct EmpireInviteClaimRequest {
    pub building_entity_id: u64,
    pub cargo_id: i32,
}

#[derive(SpacetimeType)]
pub struct EmpireLeaveRequest {
    pub building_entity_id: u64,
}

#[derive(SpacetimeType)]
pub struct EmpireDismantleRequest {
    pub building_entity_id: u64,
}

#[derive(SpacetimeType)]
pub struct EmpireUpdatePermissionsRequest {
    pub empire_entity_id: u64,
    pub rank: u8,
    pub permissions: Vec<bool>, //Based on EmpirePermission enum
}

#[derive(SpacetimeType)]
pub struct EmpireSetRankTitleRequest {
    pub empire_entity_id: u64,
    pub rank: u8,
    pub title: String,
}

#[derive(SpacetimeType)]
pub struct EmpireMarkForExpansionRequest {
    pub empire_entity_id: u64,
    pub chunk_index: u64,
    pub enabled: bool,
}

#[derive(SpacetimeType)]
pub struct EmpirePlayerJoinRequest {
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType)]
pub struct EmpirePlayerLeaveRequest {
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType)]
pub struct EmpireMarkForSiegeRequest {
    pub building_entity_id: u64,
    pub enable_siege: bool,
}

#[derive(SpacetimeType)]
pub struct EmpireSetDirectiveMessageRequest {
    pub empire_entity_id: u64,
    pub message: String,
}

#[derive(SpacetimeType)]
pub struct EmpireDonateShardsRequest {
    pub amount: u32,
    pub on_behalf_username: Option<String>,
}

#[derive(SpacetimeType)]
pub struct EmpireDonateCurrencyRequest {
    pub amount: u32,
    pub on_behalf_username: Option<String>,
}

#[derive(SpacetimeType)]
pub struct EmpireChangeEmblemRequest {
    pub empire_entity_id: u64,
    pub icon_id: i32,
    pub shape_id: i32,
    pub color1_id: i32,
    pub color2_id: i32,
}
