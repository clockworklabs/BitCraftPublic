use spacetimedb::*;

use crate::{inter_module::_autogen::{InterModuleTableUpdates, InterModuleTableUpdatesV2}, messages::game_util::ExperienceStack};

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

#[spacetimedb::table(name = inter_module_message_v5, public)]
pub struct InterModuleMessageV5 {
    #[primary_key]
    #[auto_inc]
    pub id: u64, // message id, unique for sender module
    pub to: u8, // recipient module id
    pub contents: MessageContentsV5,
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
    EmpireDonateItem(EmpireDonateItemMsg),
    EmpireCreate(EmpireCreateMsg),
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
    EmpireAddCurrency(EmpireAddCurrencyMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
    ReplaceIdentity(ReplaceIdentityMsg),
    ClaimSetName(ClaimSetNameMsg),
    RestoreSkills(RestoreSkillsMsg),
    NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg),
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
    EmpireDonateItem(EmpireDonateItemMsg),
    EmpireCreate(EmpireCreateMsg),
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
    EmpireAddCurrency(EmpireAddCurrencyMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsg),
    ReplaceIdentity(ReplaceIdentityMsg),
    ClaimSetName(ClaimSetNameMsg),
    RestoreSkills(RestoreSkillsMsg),
    NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg),
    EmpireWithdrawItem(EmpireWithdrawItemMsg),
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
    EmpireDonateItem(EmpireDonateItemMsg),
    EmpireCreate(EmpireCreateMsg),
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
    EmpireAddCurrency(EmpireAddCurrencyMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsgV2),
    ReplaceIdentity(ReplaceIdentityMsg),
    ClaimSetName(ClaimSetNameMsg),
    RestoreSkills(RestoreSkillsMsg),
    NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg),
    EmpireWithdrawItem(EmpireWithdrawItemMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContentsV4 {
    TableUpdate(InterModuleTableUpdatesV2),
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
    EmpireDonateItem(EmpireDonateItemMsg),
    EmpireCreate(EmpireCreateMsg),
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
    EmpireAddCurrency(EmpireAddCurrencyMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsgV2),
    ReplaceIdentity(ReplaceIdentityMsg),
    ClaimSetName(ClaimSetNameMsg),
    RestoreSkills(RestoreSkillsMsg),
    NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg),
    EmpireWithdrawItem(EmpireWithdrawItemMsg),
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum MessageContentsV5 {
    TableUpdate(InterModuleTableUpdatesV2),
    TransferPlayerRequest(TransferPlayerMsgV5),
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
    EmpireDonateItem(EmpireDonateItemMsg),
    EmpireCreate(EmpireCreateMsg),
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
    EmpireAddCurrency(EmpireAddCurrencyMsg),
    SignPlayerOut(SignPlayerOutMsg),
    AdminBroadcastMessage(AdminBroadcastMessageMsg),
    PlayerSkipQueue(PlayerSkipQueueMsg),
    GrantHubItem(GrantHubItemMsg),
    RecoverDeployable(RecoverDeployableMsg),
    OnDeployableRecovered(OnDeployableRecoveredMsgV2),
    ReplaceIdentity(ReplaceIdentityMsg),
    ClaimSetName(ClaimSetNameMsg),
    RestoreSkills(RestoreSkillsMsg),
    NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg),
    EmpireWithdrawItem(EmpireWithdrawItemMsg),
}

impl From<MessageContentsV4> for MessageContentsV5 {
    fn from(value: MessageContentsV4) -> Self {
        match value {
            MessageContentsV4::TableUpdate(v) => MessageContentsV5::TableUpdate(v),
            MessageContentsV4::TransferPlayerRequest(v) => MessageContentsV5::TransferPlayerRequest(v.into()),
            MessageContentsV4::TransferPlayerHousingRequest(v) => MessageContentsV5::TransferPlayerHousingRequest(v),
            MessageContentsV4::PlayerCreateRequest(v) => MessageContentsV5::PlayerCreateRequest(v),
            MessageContentsV4::UserUpdateRegionRequest(v) => MessageContentsV5::UserUpdateRegionRequest(v),
            MessageContentsV4::OnPlayerNameSetRequest(v) => MessageContentsV5::OnPlayerNameSetRequest(v),
            MessageContentsV4::ClaimCreateEmpireSettlementState(v) => MessageContentsV5::ClaimCreateEmpireSettlementState(v),
            MessageContentsV4::OnClaimMembersChanged(v) => MessageContentsV5::OnClaimMembersChanged(v),
            MessageContentsV4::EmpireCreateBuilding(v) => MessageContentsV5::EmpireCreateBuilding(v),
            MessageContentsV4::OnEmpireBuildingDeleted(v) => MessageContentsV5::OnEmpireBuildingDeleted(v),
            MessageContentsV4::GlobalDeleteEmpireBuilding(v) => MessageContentsV5::GlobalDeleteEmpireBuilding(v),
            MessageContentsV4::DeleteEmpire(v) => MessageContentsV5::DeleteEmpire(v),
            MessageContentsV4::EmpireClaimJoin(v) => MessageContentsV5::EmpireClaimJoin(v),
            MessageContentsV4::EmpireResupplyNode(v) => MessageContentsV5::EmpireResupplyNode(v),
            MessageContentsV4::EmpireDonateItem(v) => MessageContentsV5::EmpireDonateItem(v),
            MessageContentsV4::EmpireCreate(v) => MessageContentsV5::EmpireCreate(v),
            MessageContentsV4::EmpireCollectHexiteCapsule(v) => MessageContentsV5::EmpireCollectHexiteCapsule(v),
            MessageContentsV4::EmpireStartSiege(v) => MessageContentsV5::EmpireStartSiege(v),
            MessageContentsV4::EmpireSiegeAddSupplies(v) => MessageContentsV5::EmpireSiegeAddSupplies(v),
            MessageContentsV4::OnPlayerJoinedEmpire(v) => MessageContentsV5::OnPlayerJoinedEmpire(v),
            MessageContentsV4::OnPlayerLeftEmpire(v) => MessageContentsV5::OnPlayerLeftEmpire(v),
            MessageContentsV4::RegionDestroySiegeEngine(v) => MessageContentsV5::RegionDestroySiegeEngine(v),
            MessageContentsV4::OnRegionPlayerCreated(v) => MessageContentsV5::OnRegionPlayerCreated(v),
            MessageContentsV4::EmpireQueueSupplies(v) => MessageContentsV5::EmpireQueueSupplies(v),
            MessageContentsV4::EmpireUpdateEmperorCrown(v) => MessageContentsV5::EmpireUpdateEmperorCrown(v),
            MessageContentsV4::EmpireRemoveCrown(v) => MessageContentsV5::EmpireRemoveCrown(v),
            MessageContentsV4::EmpireAddCurrency(v) => MessageContentsV5::EmpireAddCurrency(v),
            MessageContentsV4::SignPlayerOut(v) => MessageContentsV5::SignPlayerOut(v),
            MessageContentsV4::AdminBroadcastMessage(v) => MessageContentsV5::AdminBroadcastMessage(v),
            MessageContentsV4::PlayerSkipQueue(v) => MessageContentsV5::PlayerSkipQueue(v),
            MessageContentsV4::GrantHubItem(v) => MessageContentsV5::GrantHubItem(v),
            MessageContentsV4::RecoverDeployable(v) => MessageContentsV5::RecoverDeployable(v),
            MessageContentsV4::OnDeployableRecovered(v) => MessageContentsV5::OnDeployableRecovered(v),
            MessageContentsV4::ReplaceIdentity(v) => MessageContentsV5::ReplaceIdentity(v),
            MessageContentsV4::ClaimSetName(v) => MessageContentsV5::ClaimSetName(v),
            MessageContentsV4::RestoreSkills(v) => MessageContentsV5::RestoreSkills(v),
            MessageContentsV4::NpcPlaceWatchtowers(v) => MessageContentsV5::NpcPlaceWatchtowers(v),
            MessageContentsV4::EmpireWithdrawItem(v) => MessageContentsV5::EmpireWithdrawItem(v),
        }
    }
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
    pub player_settings_state: Option<PlayerSettingsState>,
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
    pub equipment_preset_state: Vec<EquipmentPresetState>,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state: Vec<DeployableCollectibleState>,
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
    pub undeployed_deployable_states: Vec<DeployableState>,
    pub player_settings_state: Option<PlayerSettingsState>,
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
    //pub extract_outcome_state: ExtractOutcomeStateV2,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV3 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableStateV2>,
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
    pub equipment_preset_state: Vec<EquipmentPresetState>,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state: Vec<DeployableCollectibleState>,
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
    pub undeployed_deployable_states: Vec<DeployableStateV2>,
    pub player_settings_state: Option<PlayerSettingsState>,
    pub quest_chain_states: Vec<QuestChainState>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV4 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableStateV2>,
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
    pub equipment_preset_state: Vec<EquipmentPresetState>,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state: Vec<DeployableCollectibleState>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub ability_state: Vec<AbilityState>,
    pub action_bar_state: Vec<ActionBarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksStateV2,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub undeployed_deployable_states: Vec<DeployableStateV2>,
    pub player_settings_state: Option<PlayerSettingsState>,
    pub quest_chain_states: Vec<QuestChainState>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct TransferPlayerMsgV5 {
    pub original_location: FloatHexTileMessage,
    pub destination_location: FloatHexTileMessage,
    pub allow_cancel: bool,
    pub teleport_energy_cost: f32,

    pub vehicle: Option<DeployableStateV2>,
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
    pub equipment_preset_state: Vec<EquipmentPresetState>,
    pub inventory_state: Vec<InventoryState>,
    pub character_stats_state: CharacterStatsState,
    pub player_username_state: PlayerUsernameState,
    pub player_action_state: Vec<PlayerActionState>,
    pub deployable_collectible_state: Vec<DeployableCollectibleState>,
    pub combat_state: CombatState,
    pub action_state: Vec<ActionState>,
    pub toolbar_state: Vec<ToolbarState>,
    pub ability_state: Vec<AbilityState>,
    pub action_bar_state: Vec<ActionBarState>,
    pub attack_outcome_state: AttackOutcomeState,
    pub vault_state: VaultState,
    pub exploration_chunks_state: ExplorationChunksStateV2,
    pub satiation_state: SatiationState,
    pub player_prefs_state: PlayerPrefsState,
    pub onboarding_state: OnboardingState,
    pub unclaimed_collectibles_state: Option<UnclaimedCollectiblesState>,
    pub teleportation_energy_state: TeleportationEnergyState,
    pub player_housing_state: Option<PlayerHousingState>,
    pub traveler_task_states: Vec<TravelerTaskState>,
    pub traveler_task_credit_states: Vec<TravelerTaskCreditState>,
    pub undeployed_deployable_states: Vec<DeployableStateV2>,
    pub player_settings_state: Option<PlayerSettingsState>,
    pub quest_chain_states: Vec<QuestChainState>,
}

impl From<TransferPlayerMsgV4> for TransferPlayerMsgV5 {
    fn from(value: TransferPlayerMsgV4) -> Self {
        Self {
            original_location: value.original_location,
            destination_location: value.destination_location,
            allow_cancel: value.allow_cancel,
            teleport_energy_cost: value.teleport_energy_cost,
            vehicle: value.vehicle,
            vehicle_inventory: value.vehicle_inventory,
            player_state: value.player_state,
            user_state: value.user_state,
            move_validation_strike_counter_state: value.move_validation_strike_counter_state,
            health_state: value.health_state,
            stamina_state: value.stamina_state,
            experience_state: value.experience_state,
            active_buff_state: value.active_buff_state,
            knowledge_achievement_state: value.knowledge_achievement_state,
            knowledge_battle_action_state: value.knowledge_battle_action_state,
            knowledge_building_state: value.knowledge_building_state,
            knowledge_cargo_state: value.knowledge_cargo_state,
            knowledge_construction_state: value.knowledge_construction_state,
            knowledge_resource_placement_state: value.knowledge_resource_placement_state,
            knowledge_craft_state: value.knowledge_craft_state,
            knowledge_enemy_state: value.knowledge_enemy_state,
            knowledge_extract_state: value.knowledge_extract_state,
            knowledge_item_state: value.knowledge_item_state,
            knowledge_lore_state: value.knowledge_lore_state,
            knowledge_npc_state: value.knowledge_npc_state,
            knowledge_resource_state: value.knowledge_resource_state,
            knowledge_ruins_state: value.knowledge_ruins_state,
            knowledge_secondary_state: value.knowledge_secondary_state,
            knowledge_vault_state: value.knowledge_vault_state,
            knowledge_deployable_state: value.knowledge_deployable_state,
            knowledge_paving_state: value.knowledge_paving_state,
            knowledge_claim_state: value.knowledge_claim_state,
            knowledge_pillar_shaping_state: value.knowledge_pillar_shaping_state,
            equipment_state: value.equipment_state,
            equipment_preset_state: value.equipment_preset_state,
            inventory_state: value.inventory_state,
            character_stats_state: value.character_stats_state,
            player_username_state: value.player_username_state,
            player_action_state: value.player_action_state,
            deployable_collectible_state: value.deployable_collectible_state,
            combat_state: value.combat_state,
            action_state: value.action_state,
            toolbar_state: value.toolbar_state,
            ability_state: value.ability_state,
            action_bar_state: value.action_bar_state,
            attack_outcome_state: value.attack_outcome_state,
            vault_state: value.vault_state,
            exploration_chunks_state: value.exploration_chunks_state,
            satiation_state: value.satiation_state,
            player_prefs_state: value.player_prefs_state,
            onboarding_state: value.onboarding_state,
            unclaimed_collectibles_state: value.unclaimed_collectibles_state,
            teleportation_energy_state: value.teleportation_energy_state,
            player_housing_state: value.player_housing_state,
            traveler_task_states: value.traveler_task_states,
            traveler_task_credit_states: Vec::new(),
            undeployed_deployable_states: value.undeployed_deployable_states,
            player_settings_state: value.player_settings_state,
            quest_chain_states: value.quest_chain_states,
        }
    }
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
pub struct EmpireDonateItemMsg {
    pub player_entity_id: u64,
    pub item_id: i32,
    pub is_cargo: bool,
    pub count: u32,
    pub on_behalf_username: Option<String>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireWithdrawItemMsg {
    pub player_entity_id: u64,
    pub item_id: i32,
    pub is_cargo: bool,
    pub count: u32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct EmpireCreateMsg {
    pub player_entity_id: u64,
    pub building_entity_id: u64,
    pub empire_name: String,
    pub icon_id: i32,
    pub shape_id: i32,
    pub color1_id: i32,
    pub color2_id: i32,
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
pub struct EmpireAddCurrencyMsg {
    pub empire_entity_id: u64,
    pub amount: u32,
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
pub struct OnDeployableRecoveredMsgV2 {
    pub player_entity_id: u64,
    pub deployable_entity_id: u64,
    pub deployable_desc_id: i32,
    pub deployable_state: DeployableStateV2,
    pub trade_orders: Vec<TradeOrderState>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct ReplaceIdentityMsg {
    pub old_identity: Identity,
    pub new_identity: Identity,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct ClaimSetNameMsg {
    pub player_entity_id: u64,
    pub claim_entity_id: u64,
    pub new_name: String,
}

/// Data for a single NPC watchtower placement, including its assigned territory chunks.
#[derive(SpacetimeType, Clone, Debug)]
pub struct NpcWatchtowerPlacement {
    pub building_entity_id: u64,
    pub location: OffsetCoordinatesSmallMessage,
    pub chunk_indexes: Vec<u64>,
}

/// Inter-module message sent from region to global to create NPC empire nodes and chunk assignments.
/// Bypasses the normal player-based watchtower creation flow.
#[derive(SpacetimeType, Clone, Debug)]
pub struct NpcPlaceWatchtowersMsg {
    pub watchtowers: Vec<NpcWatchtowerPlacement>,
    pub energy: i32,
    pub upkeep: i32,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct RestoreSkillsMsg {
    pub player_entity_id: u64,
    pub experience_stacks: Vec<ExperienceStack>,
}
