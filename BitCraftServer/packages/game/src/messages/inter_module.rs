use spacetimedb::*;

use crate::inter_module::_autogen::InterModuleTableUpdates;

use super::{
    components::*,
    generic::HubItemType,
    util::{FloatHexTileMessage, OffsetCoordinatesSmallMessage},
};

//Used to check messages received by this module where it is the destination
#[spacetimedb::table(name = inter_module_message_counter)]
pub struct InterModuleMessageCounter {
    #[primary_key]
    pub module_id: u8,
    pub last_processed_message_id: u64,
}

//Used to check message responses received by this module where it is the sender
#[spacetimedb::table(name = inter_module_response_message_counter)]
pub struct InterModuleResponseMessageCounter {
    #[primary_key]
    pub dst_module_id: u8,
    pub last_processed_message_id: u64,
}

#[spacetimedb::table(name = inter_module_message_errors,
    index(name = id, btree(columns = [sender_module_id, message_id])))]
pub struct InterModuleMessageErrors {
    #[primary_key]
    pub sender_module_id: u8,
    pub message_id: u64,
    pub error: String,
}

#[spacetimedb::table(name = inter_module_message, public)]
pub struct InterModuleMessage {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // message id, unique for sender module
    pub to: u8, // recipient module id
    pub contents: MessageContents,
}

#[spacetimedb::table(name = inter_module_message_v2, public)]
pub struct InterModuleMessageV2 {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // message id, unique for sender module
    pub to: u8, // recipient module id
    pub contents: MessageContentsV2,
}

#[spacetimedb::table(name = inter_module_message_v3, public)]
pub struct InterModuleMessageV3 {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // message id, unique for sender module
    pub to: u8, // recipient module id
    pub contents: MessageContentsV3,
}

#[spacetimedb::table(name = inter_module_message_v4, public)]
pub struct InterModuleMessageV4 {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // message id, unique for sender module
    pub to: u8, // recipient module id
    pub contents: MessageContentsV4,
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContents {
    TableUpdate(InterModuleTableUpdates),
    TransferPlayerRequest(TransferPlayerMsg),
    TransferPlayerHousingRequest(TransferPlayerHousingMsg),
    PlayerCreateRequest(PlayerCreateMsg),
    UserUpdateRegionRequest(UserUpdateRegionMsg),
    OnPlayerNameSetRequest(OnPlayerNameSetMsg),
    ClaimCreateEmpireSettlementState(ClaimCreateEmpireSettlementMsg),
    OnClaimMembersChanged(OnClaimMembersChangedMsg),
    EmpireCreateBuilding(EmpireCreateBuildingMsg),
    OnEmpireBuildingDeleted(OnEmpireBuildingDeletedMsg),
    GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg),
    DeleteEmpire(DeleteEmpireMsg),
    EmpireClaimJoin(EmpireClaimJoinMsg),
    EmpireResupplyNode(EmpireResupplyNodeMsg),
    EmpireCollectHexiteCapsule(EmpireCollectHexiteCapsuleMsg),
    EmpireStartSiege(EmpireStartSiegeMsg),
    EmpireSiegeAddSupplies(EmpireSiegeAddSuppliesMsg),
    OnPlayerJoinedEmpire(OnPlayerJoinedEmpireMsg),
    OnPlayerLeftEmpire(OnPlayerLeftEmpireMsg),
    RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg),
    OnRegionPlayerCreated(OnRegionPlayerCreatedMsg),
    EmpireQueueSupplies(EmpireQueueSuppliesMsg),
    EmpireUpdateEmperorCrown(EmpireUpdateEmperorCrownMsg),
    EmpireRemoveCrown(EmpireRemoveCrownMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContentsV2 {
    TableUpdate(InterModuleTableUpdates),
    TransferPlayerRequest(TransferPlayerMsgV2),
    TransferPlayerHousingRequest(TransferPlayerHousingMsg),
    PlayerCreateRequest(PlayerCreateMsg),
    UserUpdateRegionRequest(UserUpdateRegionMsg),
    OnPlayerNameSetRequest(OnPlayerNameSetMsg),
    ClaimCreateEmpireSettlementState(ClaimCreateEmpireSettlementMsg),
    OnClaimMembersChanged(OnClaimMembersChangedMsg),
    EmpireCreateBuilding(EmpireCreateBuildingMsg),
    OnEmpireBuildingDeleted(OnEmpireBuildingDeletedMsg),
    GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg),
    DeleteEmpire(DeleteEmpireMsg),
    EmpireClaimJoin(EmpireClaimJoinMsg),
    EmpireResupplyNode(EmpireResupplyNodeMsg),
    EmpireCollectHexiteCapsule(EmpireCollectHexiteCapsuleMsg),
    EmpireStartSiege(EmpireStartSiegeMsg),
    EmpireSiegeAddSupplies(EmpireSiegeAddSuppliesMsg),
    OnPlayerJoinedEmpire(OnPlayerJoinedEmpireMsg),
    OnPlayerLeftEmpire(OnPlayerLeftEmpireMsg),
    RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg),
    OnRegionPlayerCreated(OnRegionPlayerCreatedMsg),
    EmpireQueueSupplies(EmpireQueueSuppliesMsg),
    EmpireUpdateEmperorCrown(EmpireUpdateEmperorCrownMsg),
    EmpireRemoveCrown(EmpireRemoveCrownMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContentsV3 {
    TableUpdate(InterModuleTableUpdates),
    TransferPlayerRequest(TransferPlayerMsgV3),
    TransferPlayerHousingRequest(TransferPlayerHousingMsg),
    PlayerCreateRequest(PlayerCreateMsg),
    UserUpdateRegionRequest(UserUpdateRegionMsg),
    OnPlayerNameSetRequest(OnPlayerNameSetMsg),
    ClaimCreateEmpireSettlementState(ClaimCreateEmpireSettlementMsg),
    OnClaimMembersChanged(OnClaimMembersChangedMsg),
    EmpireCreateBuilding(EmpireCreateBuildingMsg),
    OnEmpireBuildingDeleted(OnEmpireBuildingDeletedMsg),
    GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg),
    DeleteEmpire(DeleteEmpireMsg),
    EmpireClaimJoin(EmpireClaimJoinMsg),
    EmpireResupplyNode(EmpireResupplyNodeMsg),
    EmpireCollectHexiteCapsule(EmpireCollectHexiteCapsuleMsg),
    EmpireStartSiege(EmpireStartSiegeMsg),
    EmpireSiegeAddSupplies(EmpireSiegeAddSuppliesMsg),
    OnPlayerJoinedEmpire(OnPlayerJoinedEmpireMsg),
    OnPlayerLeftEmpire(OnPlayerLeftEmpireMsg),
    RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg),
    OnRegionPlayerCreated(OnRegionPlayerCreatedMsg),
    EmpireQueueSupplies(EmpireQueueSuppliesMsg),
    EmpireUpdateEmperorCrown(EmpireUpdateEmperorCrownMsg),
    EmpireRemoveCrown(EmpireRemoveCrownMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
    ReplaceIdentity(ReplaceIdentityMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContentsV4 {
    TableUpdate(InterModuleTableUpdates),
    TransferPlayerRequest(TransferPlayerMsgV4),
    TransferPlayerHousingRequest(TransferPlayerHousingMsg),
    PlayerCreateRequest(PlayerCreateMsg),
    UserUpdateRegionRequest(UserUpdateRegionMsg),
    OnPlayerNameSetRequest(OnPlayerNameSetMsg),
    ClaimCreateEmpireSettlementState(ClaimCreateEmpireSettlementMsg),
    OnClaimMembersChanged(OnClaimMembersChangedMsg),
    EmpireCreateBuilding(EmpireCreateBuildingMsg),
    OnEmpireBuildingDeleted(OnEmpireBuildingDeletedMsg),
    GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg),
    DeleteEmpire(DeleteEmpireMsg),
    EmpireClaimJoin(EmpireClaimJoinMsg),
    EmpireResupplyNode(EmpireResupplyNodeMsg),
    EmpireCollectHexiteCapsule(EmpireCollectHexiteCapsuleMsg),
    EmpireStartSiege(EmpireStartSiegeMsg),
    EmpireSiegeAddSupplies(EmpireSiegeAddSuppliesMsg),
    OnPlayerJoinedEmpire(OnPlayerJoinedEmpireMsg),
    OnPlayerLeftEmpire(OnPlayerLeftEmpireMsg),
    RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg),
    OnRegionPlayerCreated(OnRegionPlayerCreatedMsg),
    EmpireQueueSupplies(EmpireQueueSuppliesMsg),
    EmpireUpdateEmperorCrown(EmpireUpdateEmperorCrownMsg),
    EmpireRemoveCrown(EmpireRemoveCrownMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
    ReplaceIdentity(ReplaceIdentityMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsg {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableState>,
    pub vehicle_inventory: Option<InventoryState>,

    pub player_state: PlayerState,
    pub user_state: UserState,
    pub move_validation_strike_counter_state: MoveValidationStrikeCounterState,
    pub health_state: HealthState,
    pub stamina_state: StaminaState,
    pub experience_state: ExperienceState,
    pub active_buff_state: ActiveBuffState,
    pub knowledge_achievement_state: KnowledgeAchievementState,
    pub knowledge_battle_action_state: KnowledgeBattleActionState,
    pub knowledge_building_state: KnowledgeBuildingState,
    pub knowledge_cargo_state: KnowledgeCargoState,
    pub knowledge_construction_state: KnowledgeConstructionState,
    pub knowledge_resource_placement_state: KnowledgeResourcePlacementState,
    pub knowledge_craft_state: KnowledgeCraftState,
    pub knowledge_enemy_state: KnowledgeEnemyState,
    pub knowledge_extract_state: KnowledgeExtractState,
    pub knowledge_item_state: KnowledgeItemState,
    pub knowledge_lore_state: KnowledgeLoreState,
    pub knowledge_npc_state: KnowledgeNpcState,
    pub knowledge_resource_state: KnowledgeResourceState,
    pub knowledge_ruins_state: KnowledgeRuinsState,
    pub knowledge_secondary_state: KnowledgeSecondaryState,
    pub knowledge_vault_state: KnowledgeVaultState,
    pub knowledge_deployable_state: KnowledgeDeployableState,
    pub knowledge_paving_state: KnowledgePavingState,
    pub knowledge_claim_state: KnowledgeClaimState,
    pub knowledge_pillar_shaping_state: KnowledgePillarShapingState,
    pub equipment_state: EquipmentState,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state: Vec<DeployableCollectibleState>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksState,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub extract_outcome_state: ExtractOutcomeState,
    pub undeployed_deployable_states: Vec<DeployableState>,
    //
    //Player-related components that are deleted
    //
    //pub player_timestamp_state: PlayerTimestampState,
    //pub trade_session_state: TradeSessionState,
    //pub player_lowercase_username_state: PlayerLowercaseUsernameState,
    //pub mounting_state: MountingState,
    //pub target_state: TargetState,
    //pub threat_state: ThreatState,
    //pub targetable_state: TargetableState,
    //pub player_vote_state: PlayerVoteState,
    //pub alert_state: AlertState,
    //pub signed_in_player_state: SignedInPlayerState,
    //pub starving_player_state: StarvingPlayerState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV2 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableState>,
    pub vehicle_inventory: Option<InventoryState>,

    pub player_state: PlayerState,
    pub user_state: UserState,
    pub move_validation_strike_counter_state: MoveValidationStrikeCounterState,
    pub health_state: HealthState,
    pub stamina_state: StaminaState,
    pub experience_state: ExperienceState,
    pub active_buff_state: ActiveBuffState,
    pub knowledge_achievement_state: KnowledgeAchievementState,
    pub knowledge_battle_action_state: KnowledgeBattleActionState,
    pub knowledge_building_state: KnowledgeBuildingState,
    pub knowledge_cargo_state: KnowledgeCargoState,
    pub knowledge_construction_state: KnowledgeConstructionState,
    pub knowledge_resource_placement_state: KnowledgeResourcePlacementState,
    pub knowledge_craft_state: KnowledgeCraftState,
    pub knowledge_enemy_state: KnowledgeEnemyState,
    pub knowledge_extract_state: KnowledgeExtractState,
    pub knowledge_item_state: KnowledgeItemState,
    pub knowledge_lore_state: KnowledgeLoreState,
    pub knowledge_npc_state: KnowledgeNpcState,
    pub knowledge_resource_state: KnowledgeResourceState,
    pub knowledge_ruins_state: KnowledgeRuinsState,
    pub knowledge_secondary_state: KnowledgeSecondaryState,
    pub knowledge_vault_state: KnowledgeVaultState,
    pub knowledge_deployable_state: KnowledgeDeployableState,
    pub knowledge_paving_state: KnowledgePavingState,
    pub knowledge_claim_state: KnowledgeClaimState,
    pub knowledge_pillar_shaping_state: KnowledgePillarShapingState,
    pub equipment_state: EquipmentState,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state_v2: Vec<DeployableCollectibleStateV2>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksState,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub extract_outcome_state: ExtractOutcomeState,
    pub undeployed_deployable_states: Vec<DeployableState>,
    pub player_settings_state: Option<PlayerSettingsState>,
    //
    //Player-related components that are deleted
    //
    //pub player_timestamp_state: PlayerTimestampState,
    //pub trade_session_state: TradeSessionState,
    //pub player_lowercase_username_state: PlayerLowercaseUsernameState,
    //pub mounting_state: MountingState,
    //pub target_state: TargetState,
    //pub threat_state: ThreatState,
    //pub targetable_state: TargetableState,
    //pub player_vote_state: PlayerVoteState,
    //pub alert_state: AlertState,
    //pub signed_in_player_state: SignedInPlayerState,
    //pub starving_player_state: StarvingPlayerState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV3 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableState>,
    pub vehicle_inventory: Option<InventoryState>,

    pub player_state: PlayerState,
    pub user_state: UserState,
    pub move_validation_strike_counter_state: MoveValidationStrikeCounterState,
    pub health_state: HealthState,
    pub stamina_state: StaminaState,
    pub experience_state: ExperienceState,
    pub active_buff_state: ActiveBuffState,
    pub knowledge_achievement_state: KnowledgeAchievementState,
    pub knowledge_battle_action_state: KnowledgeBattleActionState,
    pub knowledge_building_state: KnowledgeBuildingState,
    pub knowledge_cargo_state: KnowledgeCargoState,
    pub knowledge_construction_state: KnowledgeConstructionState,
    pub knowledge_resource_placement_state: KnowledgeResourcePlacementState,
    pub knowledge_craft_state: KnowledgeCraftState,
    pub knowledge_enemy_state: KnowledgeEnemyState,
    pub knowledge_extract_state: KnowledgeExtractState,
    pub knowledge_item_state: KnowledgeItemState,
    pub knowledge_lore_state: KnowledgeLoreState,
    pub knowledge_npc_state: KnowledgeNpcState,
    pub knowledge_resource_state: KnowledgeResourceState,
    pub knowledge_ruins_state: KnowledgeRuinsState,
    pub knowledge_secondary_state: KnowledgeSecondaryState,
    pub knowledge_vault_state: KnowledgeVaultState,
    pub knowledge_deployable_state: KnowledgeDeployableState,
    pub knowledge_paving_state: KnowledgePavingState,
    pub knowledge_claim_state: KnowledgeClaimState,
    pub knowledge_pillar_shaping_state: KnowledgePillarShapingState,
    pub equipment_state: EquipmentState,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state_v2: Vec<DeployableCollectibleStateV2>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub ability_state: Vec<AbilityState>,
    pub action_bar_state: Vec<ActionBarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksState,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub extract_outcome_state: ExtractOutcomeState,
    pub undeployed_deployable_states: Vec<DeployableState>,
    pub player_settings_state: Option<PlayerSettingsStateV2>,
    //
    //Player-related components that are deleted
    //
    //pub player_timestamp_state: PlayerTimestampState,
    //pub trade_session_state: TradeSessionState,
    //pub player_lowercase_username_state: PlayerLowercaseUsernameState,
    //pub mounting_state: MountingState,
    //pub target_state: TargetState,
    //pub threat_state: ThreatState,
    //pub targetable_state: TargetableState,
    //pub player_vote_state: PlayerVoteState,
    //pub alert_state: AlertState,
    //pub signed_in_player_state: SignedInPlayerState,
    //pub starving_player_state: StarvingPlayerState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV4 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableState>,
    pub vehicle_inventory: Option<InventoryState>,

    pub player_state: PlayerState,
    pub user_state: UserState,
    pub move_validation_strike_counter_state: MoveValidationStrikeCounterState,
    pub health_state: HealthState,
    pub stamina_state: StaminaState,
    pub experience_state: ExperienceState,
    pub active_buff_state: ActiveBuffState,
    pub knowledge_achievement_state: KnowledgeAchievementState,
    pub knowledge_battle_action_state: KnowledgeBattleActionState,
    pub knowledge_building_state: KnowledgeBuildingState,
    pub knowledge_cargo_state: KnowledgeCargoState,
    pub knowledge_construction_state: KnowledgeConstructionState,
    pub knowledge_resource_placement_state: KnowledgeResourcePlacementState,
    pub knowledge_craft_state: KnowledgeCraftState,
    pub knowledge_enemy_state: KnowledgeEnemyState,
    pub knowledge_extract_state: KnowledgeExtractState,
    pub knowledge_item_state: KnowledgeItemState,
    pub knowledge_lore_state: KnowledgeLoreState,
    pub knowledge_npc_state: KnowledgeNpcState,
    pub knowledge_resource_state: KnowledgeResourceState,
    pub knowledge_ruins_state: KnowledgeRuinsState,
    pub knowledge_secondary_state: KnowledgeSecondaryState,
    pub knowledge_vault_state: KnowledgeVaultState,
    pub knowledge_deployable_state: KnowledgeDeployableState,
    pub knowledge_paving_state: KnowledgePavingState,
    pub knowledge_claim_state: KnowledgeClaimState,
    pub knowledge_pillar_shaping_state: KnowledgePillarShapingState,
    pub equipment_state: EquipmentState,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state_v2: Vec<DeployableCollectibleStateV2>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub ability_state: Vec<AbilityState>,
    pub action_bar_state: Vec<ActionBarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksState,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub extract_outcome_state: ExtractOutcomeState,
    pub undeployed_deployable_states: Vec<DeployableState>,
    pub player_settings_state: Option<PlayerSettingsStateV2>,
    pub quest_chain_states: Vec<QuestChainState>,
    //
    //Player-related components that are deleted
    //
    //pub player_timestamp_state: PlayerTimestampState,
    //pub trade_session_state: TradeSessionState,
    //pub player_lowercase_username_state: PlayerLowercaseUsernameState,
    //pub mounting_state: MountingState,
    //pub target_state: TargetState,
    //pub threat_state: ThreatState,
    //pub targetable_state: TargetableState,
    //pub player_vote_state: PlayerVoteState,
    //pub alert_state: AlertState,
    //pub signed_in_player_state: SignedInPlayerState,
    //pub starving_player_state: StarvingPlayerState,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerHousingMsg {
    pub player_entity_id: u64,
    pub network_entity_id: u64,
    pub interior_portal_entity_id: u64,
    pub new_entrance_building_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct PlayerCreateMsg {
    pub identity: Identity,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct UserUpdateRegionMsg {
    pub identity: Identity,
    pub region_id: u8,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnPlayerNameSetMsg {
    pub player_entity_id: u64,
    pub name: String,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct ClaimCreateEmpireSettlementMsg {
    pub claim_entity_id: u64,
    pub building_entity_id: u64,
    pub location: OffsetCoordinatesSmallMessage,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnClaimMembersChangedMsg {
    pub claim_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct DestroyEmpireBuilding {
    pub building_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireCreateBuildingMsg {
    pub location: OffsetCoordinatesSmallMessage,
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub building_desc_id: i32,
    pub construction_recipe_id: Option<i32>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct GlobalDeleteEmpireBuildingMsg {
    pub player_entity_id: u64,
    pub building_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct DeleteEmpireMsg {
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnEmpireBuildingDeletedMsg {
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub ignore_portals: bool,
    pub drop_items: bool,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireClaimJoinMsg {
    pub player_entity_id: u64,
    pub claim_entity_id: u64,
    pub claim_building_entity_id: u64,
    pub empire_entity_id: u64,
    pub claim_name: String,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireResupplyNodeMsg {
    pub building_entity_id: u64,
    pub supplies_count: i32,
    pub player_entity_id: u64,
    pub cargo_id: i32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireCollectHexiteCapsuleMsg {
    pub building_entity_id: u64,
    pub player_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireStartSiegeMsg {
    pub building_coord: OffsetCoordinatesSmallMessage,
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub deployable_entity_id: u64,
    pub supplies: i32,
    pub supply_cargo_id: i32,
    pub is_depleted_watchtower: bool,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireSiegeAddSuppliesMsg {
    pub siege_entity_id: u64,
    pub player_entity_id: u64,
    pub supplies: i32,
    pub supply_cargo_id: i32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireQueueSuppliesMsg {
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub claim_entity_id: u64,
    pub claim_building_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnPlayerJoinedEmpireMsg {
    pub player_entity_id: u64,
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnPlayerLeftEmpireMsg {
    pub player_entity_id: u64,
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct RegionDestroySiegeEngineMsg {
    pub deployable_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnRegionPlayerCreatedMsg {
    pub player_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireUpdateEmperorCrownMsg {
    pub empire_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireRemoveCrownMsg {
    pub player_entity_id: u64,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct SignPlayerOutMsg {
    pub player_identity: Identity,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct AdminBroadcastMessageMsg {
    pub title: String,
    pub message: String,
    pub sign_out: bool,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct PlayerSkipQueueMsg {
    pub player_identity: Identity,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct GrantHubItemMsg {
    pub player_identity: Identity,
    pub item_type: HubItemType,
    pub item_id: i32,
    pub quantity: u32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct RecoverDeployableMsg {
    pub player_entity_id: u64,
    pub deployable_entity_id: u64,
    pub deployable_desc_id: i32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct OnDeployableRecoveredMsg {
    pub player_entity_id: u64,
    pub deployable_entity_id: u64,
    pub deployable_desc_id: i32,
    pub deployable_state: DeployableState,
    pub trade_orders: Vec<TradeOrderState>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct ReplaceIdentityMsg {
    pub old_identity: Identity,
    pub new_identity: Identity,
}
