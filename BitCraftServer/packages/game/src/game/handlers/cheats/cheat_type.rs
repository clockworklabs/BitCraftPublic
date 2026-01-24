use spacetimedb::{Identity, ReducerContext};

use crate::messages::{
    authentication::{identity_role, Role},
    generic::config,
};

pub enum CheatType {
    CheatBuildingPlace,
    CheatBuildingMove,
    CheatCargoGrant,
    CheatCompendiumPlaceEnemy,
    CheatCompendiumPlaceResource,
    CheatDiscoverMap,
    CheatExperienceGrant,
    CheatShardsGrant,
    CheatGrantKnowledge,
    CheatItemStackGrant,
    CheatPavingAddTile,
    CheatPavingDestroy,
    CheatPillarShapingAddPillar,
    CheatPillarShapingDestroy,
    CheatQuestClearState,
    CheatRemoveEntityBuilding,
    CheatRemoveEntityEnemy,
    CheatRemoveEntityResource,
    CheatSpawnLootChest,
    CheatTeleportFloat,
    CheatTerraform,
    CheatToggleActiveCollectible,
    CheatWarp,
    CheatProjectSiteAddAllMaterials,
    CheatUserSetName,
    CheatDeleteItem,
    CheatDeployableStore,
    CheatClaimTotemAddSupplies,
    CheatClaimTotemResearchAll,
    CheatClaimTotemCurrentResearch,
    CheatClaimTotemUnlockTech,
    CheatClaimsCompleteAllCurrentResearch,
    CheatClearBuffsAndDebuffs,
    CheatClaimTakeOwnership,
    CheatKill,
    CheatGrantTeleportEnergy,
    CheatClaimDeleteWalls,
    CheatSkipQuest,
    CheatRestartQuest,
    CheatAdvanceQuestToHandIn,
    CheatSkipQuestStage,
}

pub fn can_run_cheat(ctx: &ReducerContext, identity: &Identity, cheat_type: CheatType) -> bool {
    match ctx.db.config().version().find(&0) {
        Some(config) => {
            if config.env == "dev" {
                return true; // dev config allows all operations
            }
        }
        None => {}
    }

    let role = match ctx.db.identity_role().identity().find(identity) {
        Some(entry) => entry.role,
        None => return false,
    };

    match cheat_type {
        CheatType::CheatBuildingPlace => role as i32 >= Role::Gm as i32,
        CheatType::CheatBuildingMove => role as i32 >= Role::Gm as i32,
        CheatType::CheatCargoGrant => role as i32 >= Role::Gm as i32,
        CheatType::CheatCompendiumPlaceEnemy => role as i32 >= Role::Gm as i32,
        CheatType::CheatCompendiumPlaceResource => role as i32 >= Role::Gm as i32,
        CheatType::CheatDiscoverMap => role as i32 >= Role::Gm as i32,
        CheatType::CheatExperienceGrant => role as i32 >= Role::Gm as i32,
        CheatType::CheatShardsGrant => role as i32 >= Role::Gm as i32,
        CheatType::CheatGrantKnowledge => role as i32 >= Role::Gm as i32,
        CheatType::CheatItemStackGrant => role as i32 >= Role::Gm as i32,
        CheatType::CheatPavingAddTile => role as i32 >= Role::Gm as i32,
        CheatType::CheatPavingDestroy => role as i32 >= Role::Mod as i32,
        CheatType::CheatPillarShapingAddPillar => role as i32 >= Role::Gm as i32,
        CheatType::CheatPillarShapingDestroy => role as i32 >= Role::Mod as i32,
        CheatType::CheatQuestClearState => role as i32 >= Role::Mod as i32,
        CheatType::CheatRemoveEntityBuilding => role as i32 >= Role::Mod as i32,
        CheatType::CheatRemoveEntityEnemy => role as i32 >= Role::Mod as i32,
        CheatType::CheatRemoveEntityResource => role as i32 >= Role::Mod as i32,
        CheatType::CheatSpawnLootChest => role as i32 >= Role::Gm as i32,
        CheatType::CheatTeleportFloat => role as i32 >= Role::Mod as i32,
        CheatType::CheatTerraform => role as i32 >= Role::Gm as i32,
        CheatType::CheatToggleActiveCollectible => role as i32 >= Role::Gm as i32,
        CheatType::CheatWarp => role as i32 >= Role::Mod as i32,
        CheatType::CheatProjectSiteAddAllMaterials => role as i32 >= Role::Mod as i32,
        CheatType::CheatUserSetName => role as i32 >= Role::Mod as i32,
        CheatType::CheatGrantTeleportEnergy => role as i32 >= Role::Gm as i32,

        CheatType::CheatDeleteItem => role as i32 >= Role::Gm as i32,
        CheatType::CheatDeployableStore => role as i32 >= Role::Gm as i32,

        CheatType::CheatClaimTotemAddSupplies => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimTotemResearchAll => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimTotemCurrentResearch => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimTotemUnlockTech => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimsCompleteAllCurrentResearch => role as i32 >= Role::Gm as i32,
        CheatType::CheatClearBuffsAndDebuffs => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimTakeOwnership => role as i32 >= Role::Gm as i32,
        CheatType::CheatKill => role as i32 >= Role::Gm as i32,
        CheatType::CheatClaimDeleteWalls => role as i32 >= Role::Gm as i32,
        CheatType::CheatSkipQuest => role as i32 >= Role::Gm as i32,
        CheatType::CheatRestartQuest => role as i32 >= Role::Gm as i32,
        CheatType::CheatAdvanceQuestToHandIn => role as i32 >= Role::Gm as i32,
        CheatType::CheatSkipQuestStage => role as i32 >= Role::Gm as i32,
    }
}
