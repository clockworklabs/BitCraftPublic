use crate::game::handlers::attack::AttackImpactTimerMigrated;
use crate::game::handlers::player::player_death::PlayerDeathTimer;
use crate::messages::action_request::{
    ClaimResupplyRequest, DeployableDeployRequest, DeployableStoreRequest, EnemyMoveRequest, EntityAttackRequest,
    PlayerBuildingDeconstructRequest, PlayerBuildingRepairRequest, PlayerClimbRequest, PlayerCraftContinueRequest,
    PlayerCraftInitiateRequest, PlayerDeployableMoveRequest, PlayerEmoteRequest, PlayerExtractRequest, PlayerItemConvertRequest,
    PlayerMoveRequest, PlayerPavingDestroyTileRequest, PlayerPavingPlaceTileRequest, PlayerPillarShapingDestroyRequest,
    PlayerPillarShapingPlaceRequest, PlayerPlaceableInteractRequest, PlayerPlaceablePlaceRequest, PlayerProjectSiteAdvanceProjectRequest,
    PlayerSetHomeRequest, PlayerSleepRequest, PlayerTeleportHomeRequest, PlayerTeleportWaystoneRequest, PlayerTerraformRequest,
    ServerTeleportReason,
};
use crate::messages::components::NotificationSeverity;
use crate::messages::empire_shared::EmpireResupplyNodeRequest;
use crate::messages::static_data::EnemyType;
use crate::messages::util::{OffsetCoordinatesFloat, SmallHexTileMessage};
use bitcraft_macro::event_table;
use spacetimedb::{Identity, SpacetimeType};

/// Ephemeral notifications for effects that must be visible to connections other
/// than the reducer caller in SpacetimeDB 2.x.
#[spacetimedb::table(accessor = player_move_event, public, event)]
pub struct PlayerMoveEvent {
    pub actor_id: u64,
    pub request: PlayerMoveRequest,
}

#[spacetimedb::table(accessor = deployable_move_event, public, event)]
pub struct DeployableMoveEvent {
    pub actor_id: u64,
    pub request: PlayerDeployableMoveRequest,
    pub is_follow: bool,
}

#[spacetimedb::table(accessor = entity_attack_start_event, public, event)]
pub struct EntityAttackStartEvent {
    pub request: EntityAttackRequest,
}

#[spacetimedb::table(accessor = extract_start_event, public, event)]
pub struct ExtractStartEvent {
    pub actor_id: u64,
    pub request: PlayerExtractRequest,
}

#[spacetimedb::table(accessor = craft_initiate_start_event, public, event)]
pub struct CraftInitiateStartEvent {
    pub actor_id: u64,
    pub request: PlayerCraftInitiateRequest,
}

#[spacetimedb::table(accessor = craft_continue_start_event, public, event)]
pub struct CraftContinueStartEvent {
    pub actor_id: u64,
    pub request: PlayerCraftContinueRequest,
}

#[spacetimedb::table(accessor = project_site_advance_project_start_event, public, event)]
pub struct ProjectSiteAdvanceProjectStartEvent {
    pub actor_id: u64,
    pub request: PlayerProjectSiteAdvanceProjectRequest,
}

#[spacetimedb::table(accessor = terraform_start_event, public, event)]
pub struct TerraformStartEvent {
    pub actor_id: u64,
    pub request: PlayerTerraformRequest,
}

#[spacetimedb::table(accessor = building_repair_start_event, public, event)]
pub struct BuildingRepairStartEvent {
    pub actor_id: u64,
    pub request: PlayerBuildingRepairRequest,
}

#[spacetimedb::table(accessor = building_deconstruct_start_event, public, event)]
pub struct BuildingDeconstructStartEvent {
    pub actor_id: u64,
    pub request: PlayerBuildingDeconstructRequest,
}

#[spacetimedb::table(accessor = claim_resupply_start_event, public, event)]
pub struct ClaimResupplyStartEvent {
    pub actor_id: u64,
    pub request: ClaimResupplyRequest,
}

#[spacetimedb::table(accessor = empire_resupply_node_start_event, public, event)]
pub struct EmpireResupplyNodeStartEvent {
    pub actor_id: u64,
    pub request: EmpireResupplyNodeRequest,
}

#[spacetimedb::table(accessor = item_convert_start_event, public, event)]
pub struct ItemConvertStartEvent {
    pub actor_id: u64,
    pub request: PlayerItemConvertRequest,
}

#[spacetimedb::table(accessor = emote_start_event, public, event)]
pub struct EmoteStartEvent {
    pub actor_id: u64,
    pub request: PlayerEmoteRequest,
}

#[spacetimedb::table(accessor = deployable_deploy_start_event, public, event)]
pub struct DeployableDeployStartEvent {
    pub actor_id: u64,
    pub request: DeployableDeployRequest,
}

#[spacetimedb::table(accessor = deployable_store_start_event, public, event)]
pub struct DeployableStoreStartEvent {
    pub actor_id: u64,
    pub request: DeployableStoreRequest,
}

#[spacetimedb::table(accessor = placeable_place_start_event, public, event)]
pub struct PlaceablePlaceStartEvent {
    pub actor_id: u64,
    pub request: PlayerPlaceablePlaceRequest,
}

#[spacetimedb::table(accessor = placeable_interact_start_event, public, event)]
pub struct PlaceableInteractStartEvent {
    pub actor_id: u64,
    pub request: PlayerPlaceableInteractRequest,
}

#[spacetimedb::table(accessor = paving_place_tile_start_event, public, event)]
pub struct PavingPlaceTileStartEvent {
    pub actor_id: u64,
    pub request: PlayerPavingPlaceTileRequest,
}

#[spacetimedb::table(accessor = paving_destroy_tile_start_event, public, event)]
pub struct PavingDestroyTileStartEvent {
    pub actor_id: u64,
    pub request: PlayerPavingDestroyTileRequest,
}

#[spacetimedb::table(accessor = pillar_shaping_place_pillar_start_event, public, event)]
pub struct PillarShapingPlacePillarStartEvent {
    pub actor_id: u64,
    pub request: PlayerPillarShapingPlaceRequest,
}

#[spacetimedb::table(accessor = pillar_shaping_destroy_start_event, public, event)]
pub struct PillarShapingDestroyStartEvent {
    pub actor_id: u64,
    pub request: PlayerPillarShapingDestroyRequest,
}

#[spacetimedb::table(accessor = player_climb_start_event, public, event)]
pub struct PlayerClimbStartEvent {
    pub actor_id: u64,
    pub request: PlayerClimbRequest,
}

#[spacetimedb::table(accessor = player_teleport_home_start_event, public, event)]
pub struct PlayerTeleportHomeStartEvent {
    pub actor_id: u64,
    pub request: PlayerTeleportHomeRequest,
}

#[spacetimedb::table(accessor = player_teleport_waystone_start_event, public, event)]
pub struct PlayerTeleportWaystoneStartEvent {
    pub actor_id: u64,
    pub request: PlayerTeleportWaystoneRequest,
}

#[spacetimedb::table(accessor = prospect_start_event, public, event)]
pub struct ProspectStartEvent {
    pub actor_id: u64,
    pub prospecting_id: i32,
    pub timestamp: u64,
}

#[spacetimedb::table(accessor = sleep_event, public, event)]
pub struct SleepEvent {
    pub actor_id: u64,
    pub request: PlayerSleepRequest,
}

#[spacetimedb::table(accessor = set_home_event, public, event)]
pub struct SetHomeEvent {
    pub actor_id: u64,
    pub request: PlayerSetHomeRequest,
}

#[spacetimedb::table(accessor = player_death_start_event, public, event)]
pub struct PlayerDeathStartEvent {
    pub actor_id: u64,
    pub timer: PlayerDeathTimer,
}

#[spacetimedb::table(accessor = attack_impact_event, public, event)]
pub struct AttackImpactEvent {
    pub timer: AttackImpactTimerMigrated,
}

#[spacetimedb::table(accessor = attack_event, public, event)]
#[derive(Clone)]
pub struct AttackEvent {
    pub attacker_entity_id: u64,
    pub defender_entity_id: u64,
    pub combat_action_id: i32,
    pub damage: i32,
    pub is_crit: bool,
    pub is_dodge: bool,
}

#[spacetimedb::table(accessor = enemy_move_event, public, event)]
pub struct EnemyMoveEvent {
    pub request: EnemyMoveRequest,
}

#[spacetimedb::table(accessor = enemy_despawn_event, public, event)]
pub struct EnemyDespawnEvent {
    pub entity_id: u64,
    pub enemy_type: EnemyType,
    pub despawn_timestamp: u64,
    pub location: OffsetCoordinatesFloat,
    pub destination: OffsetCoordinatesFloat,
    pub movement_timestamp: u64,
}

#[spacetimedb::table(accessor = extract_event, public, event)]
#[derive(Clone)]
pub struct ExtractEvent {
    pub actor_entity_id: u64,
    pub target_entity_id: u64,
    pub damage: i32,
    pub is_crit: bool,
}

/// Signals that an extraction action depleted a resource and its client
/// representation should be kept alive long enough to play its deplete effect.
#[spacetimedb::table(accessor = resource_depleted_event, public, event)]
pub struct ResourceDepletedEvent {
    pub player_entity_id: u64,
    pub resource_entity_id: u64,
    pub location: SmallHexTileMessage,
    pub show_time_left: bool,
}

#[derive(SpacetimeType, Copy, Clone)]
#[repr(i32)]
pub enum PlayerSignedOutReason {
    Inactivity = 0,
    AdminAction,
    ServerAction,
}

#[spacetimedb::table(accessor = player_signed_out_event, public, event)]
pub struct PlayerSignedOutEvent {
    pub identity: Identity,
    pub reason: PlayerSignedOutReason,
}

#[spacetimedb::table(accessor = player_teleport_event, public, event)]
pub struct PlayerTeleportEvent {
    pub actor_id: u64,
}

#[spacetimedb::table(accessor = server_teleport_event, public, event)]
pub struct ServerTeleportEvent {
    pub player_entity_id: u64,
    pub reason: ServerTeleportReason,
}

#[spacetimedb::table(accessor = player_death_event, public, event)]
pub struct PlayerDeathEvent {
    pub player_entity_id: u64,
}

#[spacetimedb::table(accessor = deployable_disembark_event, public, event)]
pub struct DeployableDisembarkEvent {
    pub actor_id: u64,
    pub deployable_entity_id: u64,
}

#[spacetimedb::table(accessor = deployable_mount_event, public, event)]
pub struct DeployableMountEvent {
    pub actor_id: u64,
    pub deployable_entity_id: u64,
}

#[event_table(name = player_notification_event)]
pub struct PlayerNotificationEvent {
    pub player_entity_id: u64,
    pub message: String,
    pub severity: NotificationSeverity,
}

#[event_table(name = player_region_transfer_event)]
pub struct PlayerRegionTransferEvent {
    pub player_entity_id: u64,
    pub new_region_index: u8,
}

#[event_table(name = player_set_name_outcome_event)]
pub struct PlayerSetNameOutcomeEvent {
    pub player_entity_id: u64,
}

#[spacetimedb::table(accessor = building_buff_activate_event, public, event)]
pub struct BuildingBuffActivateEvent {
    pub actor_entity_id: u64,
    pub building_entity_id: u64,
    pub empire_entity_id: u64,
    pub amount: u32,
}