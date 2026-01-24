use std::collections::HashMap;

use crate::agents;
use crate::game::autogen::_static_data::{clear_staged_static_data, validate_staged_data};
use crate::game::handlers::admin::admin_update_light_source_states::admin_update_light_source_states;
use crate::game::handlers::authentication::has_role;
use crate::game::handlers::migration::migrate_achievements::migrate_achievements;
use crate::game::world_gen::resources_log::{resources_log, ResourcesLog};
use crate::messages::authentication::{identity_role, server_identity, IdentityRole, Role, ServerIdentity};
use crate::messages::components::BuildingState;
use crate::messages::generic::{
    admin_broadcast, config, globals, resource_count, world_region_name_state, world_region_state, AdminBroadcast, ResourceCount,
    WorldRegionNameState, WorldRegionState,
};
use crate::messages::generic::{Config, Globals};
use spacetimedb::{log, ReducerContext, Table};

use crate::game::location_cache::*;
use crate::messages::components::*;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::static_data::*;

#[spacetimedb::reducer]
pub fn import_skill_desc(ctx: &ReducerContext, records: Vec<SkillDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_skill_desc_internal(ctx, records)?;
    Ok(())
}
fn import_skill_desc_internal(ctx: &ReducerContext, records: Vec<SkillDesc>) -> Result<(), String> {
    for id in ctx.db.skill_desc().iter().map(|item| item.id) {
        ctx.db.skill_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type SkillDesc", len);
    for record in records {
        let record_skill = SkillType::to_enum(record.skill_type);
        if record_skill == SkillType::None {
            return Err(format!("Skill '{}' (id {}) has type 'None'", record.name, record.id).into());
        }
        if record_skill as i32 != record.id {
            return Err(format!("Skill '{}' id doesn't match type", record.name).into());
        }
        let id = record.id;
        if let Err(err) = ctx.db.skill_desc().try_insert(record) {
            return Err(format!("Couldn't insert SkillDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type SkillDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_achievement_desc(ctx: &ReducerContext, records: Vec<AchievementDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_achievement_desc_internal(ctx, records)?;
    Ok(())
}
fn import_achievement_desc_internal(ctx: &ReducerContext, records: Vec<AchievementDesc>) -> Result<(), String> {
    for id in ctx.db.achievement_desc().iter().map(|item| item.id) {
        ctx.db.achievement_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type AchievementDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.achievement_desc().try_insert(record) {
            return Err(format!("Couldn't insert AchievementDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type AchievementDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_knowledge_stat_modifier_desc(ctx: &ReducerContext, records: Vec<KnowledgeStatModifierDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_knowledge_stat_modifier_desc_internal(ctx, records)?;
    Ok(())
}
fn import_knowledge_stat_modifier_desc_internal(ctx: &ReducerContext, records: Vec<KnowledgeStatModifierDesc>) -> Result<(), String> {
    for id in ctx.db.knowledge_stat_modifier_desc().iter().map(|item| item.secondary_knowledge_id) {
        ctx.db.knowledge_stat_modifier_desc().secondary_knowledge_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type KnowledgeStatModifierDesc", len);
    for record in records {
        let id = record.secondary_knowledge_id;
        if let Err(err) = ctx.db.knowledge_stat_modifier_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert KnowledgeStatModifierDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type KnowledgeStatModifierDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_active_buff_state(ctx: &ReducerContext, records: Vec<ActiveBuffState>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_active_buff_state_internal(ctx, records)?;
    Ok(())
}
fn import_active_buff_state_internal(ctx: &ReducerContext, records: Vec<ActiveBuffState>) -> Result<(), String> {
    for id in ctx.db.active_buff_state().iter().map(|item| item.entity_id) {
        ctx.db.active_buff_state().entity_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ActiveBuffState", len);
    for record in records {
        let id = record.entity_id;
        if let Err(err) = ctx.db.active_buff_state().try_insert(record) {
            return Err(format!(
                "Couldn't insert ActiveBuffState record with entity_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ActiveBuffState", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_admin_broadcast(ctx: &ReducerContext, records: Vec<AdminBroadcast>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type AdminBroadcast", records.len());
    let len = records.len();
    for record in records {
        ctx.db.admin_broadcast().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type AdminBroadcast", len);
}

#[spacetimedb::reducer]
pub fn import_alert_desc(ctx: &ReducerContext, records: Vec<AlertDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_alert_desc_internal(ctx, records)?;
    Ok(())
}
fn import_alert_desc_internal(ctx: &ReducerContext, records: Vec<AlertDesc>) -> Result<(), String> {
    for id in ctx.db.alert_desc().iter().map(|item| item.alert_type) {
        ctx.db.alert_desc().alert_type().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type AlertDesc", len);
    for record in records {
        let id = record.alert_type;
        if let Err(err) = ctx.db.alert_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert AlertDesc record with alert_type {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type AlertDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_alert_state(ctx: &ReducerContext, records: Vec<AlertState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type AlertState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.alert_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type AlertState", len);
}
#[spacetimedb::reducer]
pub fn import_attack_outcome_state(ctx: &ReducerContext, records: Vec<AttackOutcomeState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type AttackOutcomeState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.attack_outcome_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type AttackOutcomeState", len);
}

#[spacetimedb::reducer]
pub fn import_biome_desc(ctx: &ReducerContext, records: Vec<BiomeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_biome_desc_internal(ctx, records)?;
    Ok(())
}
fn import_biome_desc_internal(ctx: &ReducerContext, records: Vec<BiomeDesc>) -> Result<(), String> {
    for id in ctx.db.biome_desc().iter().map(|item| item.biome_type) {
        ctx.db.biome_desc().biome_type().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BiomeDesc", len);
    for record in records {
        let id = record.biome_type;
        if let Err(err) = ctx.db.biome_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert BiomeDesc record with biome_type {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type BiomeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_buff_type_desc(ctx: &ReducerContext, records: Vec<BuffTypeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_buff_type_desc_internal(ctx, records)?;
    Ok(())
}
fn import_buff_type_desc_internal(ctx: &ReducerContext, records: Vec<BuffTypeDesc>) -> Result<(), String> {
    for id in ctx.db.buff_type_desc().iter().map(|item| item.id) {
        ctx.db.buff_type_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuffTypeDesc", len);
    for record in records {
        //Make sure buffs with special logic are defined coorectly
        fn validate_id(record: &BuffTypeDesc, category: BuffCategory) {
            if record.id != category as i32 {
                panic!(
                    "Buff Type '{}' (id {}) has category '{:?}', so its id must be {}",
                    record.name, record.id, category, category as i32
                );
            }
        }
        let record_category = BuffCategory::to_enum(record.category);

        match record_category {
            BuffCategory::None => panic!("Category not set for Buff Type '{}' (id {})", record.name, record.id),
            category => {
                if category.has_only_one_buff() {
                    validate_id(&record, category)
                }
            }
        };

        let id = record.id;
        if let Err(err) = ctx.db.buff_type_desc().try_insert(record) {
            return Err(format!("Couldn't insert BuffTypeDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type BuffTypeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_buff_desc(ctx: &ReducerContext, records: Vec<BuffDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_buff_desc_internal(ctx, records)?;
    Ok(())
}
fn import_buff_desc_internal(ctx: &ReducerContext, records: Vec<BuffDesc>) -> Result<(), String> {
    for id in ctx.db.buff_desc().iter().map(|item| item.id) {
        ctx.db.buff_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuffDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.buff_desc().try_insert(record) {
            return Err(format!("Couldn't insert BuffDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type BuffDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_claim_desc(ctx: &ReducerContext, records: Vec<BuildingClaimDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_claim_desc_internal(ctx, records)?;
    Ok(())
}
fn import_building_claim_desc_internal(ctx: &ReducerContext, records: Vec<BuildingClaimDesc>) -> Result<(), String> {
    for id in ctx.db.building_claim_desc().iter().map(|item| item.building_id) {
        ctx.db.building_claim_desc().building_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingClaimDesc", len);
    for record in records {
        let id = record.building_id;
        if let Err(err) = ctx.db.building_claim_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert BuildingClaimDesc record with building_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type BuildingClaimDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_desc(ctx: &ReducerContext, records: Vec<BuildingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_desc_internal(ctx, records)?;
    Ok(())
}
fn import_building_desc_internal(ctx: &ReducerContext, records: Vec<BuildingDesc>) -> Result<(), String> {
    let old_descs: HashMap<i32, BuildingDesc> = ctx.db.building_desc().iter().map(|d| (d.id, d)).collect();
    for (&id, _) in &old_descs {
        ctx.db.building_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.building_desc().try_insert(record) {
            return Err(format!("Couldn't insert BuildingDesc record with id {id}. Error message: {err}"));
        }
    }
    crate::game::handlers::migration::migration_static_data::migrate_health_of_existing_buildings(ctx, old_descs)?;
    log::info!("Inserted {} records of type BuildingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_portal_desc_v2(ctx: &ReducerContext, records: Vec<BuildingPortalDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_portal_desc_v2_internal(ctx, records)?;
    Ok(())
}
fn import_building_portal_desc_v2_internal(ctx: &ReducerContext, records: Vec<BuildingPortalDescV2>) -> Result<(), String> {
    for id in ctx.db.building_portal_desc_v2().iter().map(|item| item.id) {
        ctx.db.building_portal_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingPortalDescV2", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.building_portal_desc_v2().try_insert(record) {
            return Err(format!(
                "Couldn't insert BuildingPortalDescV2 record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type BuildingPortalDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_repairs_desc(ctx: &ReducerContext, records: Vec<BuildingRepairsDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_repairs_desc_internal(ctx, records)?;
    Ok(())
}
fn import_building_repairs_desc_internal(ctx: &ReducerContext, records: Vec<BuildingRepairsDesc>) -> Result<(), String> {
    for id in ctx.db.building_repairs_desc().iter().map(|item| item.cargo_id) {
        ctx.db.building_repairs_desc().cargo_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingRepairsDesc", len);
    for record in records {
        let id = record.cargo_id;
        if let Err(err) = ctx.db.building_repairs_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert BuildingRepairsDesc record with cargo_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type BuildingRepairsDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_spawn_desc(ctx: &ReducerContext, records: Vec<BuildingSpawnDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_spawn_desc_internal(ctx, records)?;
    Ok(())
}
fn import_building_spawn_desc_internal(ctx: &ReducerContext, records: Vec<BuildingSpawnDesc>) -> Result<(), String> {
    for id in ctx.db.building_spawn_desc().iter().map(|item| item.id) {
        ctx.db.building_spawn_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingSpawnDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.building_spawn_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert BuildingSpawnDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type BuildingSpawnDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_building_state(ctx: &ReducerContext, records: Vec<BuildingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type BuildingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.building_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type BuildingState", len);
}

#[spacetimedb::reducer]
pub fn import_building_type_desc(ctx: &ReducerContext, records: Vec<BuildingTypeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_building_type_desc_internal(ctx, records)?;
    Ok(())
}
fn import_building_type_desc_internal(ctx: &ReducerContext, records: Vec<BuildingTypeDesc>) -> Result<(), String> {
    for id in ctx.db.building_type_desc().iter().map(|item| item.id) {
        ctx.db.building_type_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type BuildingTypeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.building_type_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert BuildingTypeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type BuildingTypeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_cargo_desc(ctx: &ReducerContext, records: Vec<CargoDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_cargo_desc_internal(ctx, records)?;
    Ok(())
}
fn import_cargo_desc_internal(ctx: &ReducerContext, records: Vec<CargoDesc>) -> Result<(), String> {
    for id in ctx.db.cargo_desc().iter().map(|item| item.id) {
        ctx.db.cargo_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CargoDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.cargo_desc().try_insert(record) {
            return Err(format!("Couldn't insert CargoDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type CargoDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_character_stat_desc(ctx: &ReducerContext, records: Vec<CharacterStatDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_character_stat_desc_internal(ctx, records)?;
    Ok(())
}
fn import_character_stat_desc_internal(ctx: &ReducerContext, records: Vec<CharacterStatDesc>) -> Result<(), String> {
    for id in ctx.db.character_stat_desc().iter().map(|item: CharacterStatDesc| item.stat_type) {
        ctx.db.character_stat_desc().stat_type().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CharacterStatDesc", len);
    for record in records {
        let id = record.stat_type;
        if let Err(err) = ctx.db.character_stat_desc().try_insert(record) {
            let err = format!(
                "Couldn't insert CharacterStatDesc record with stat_type {:?}. Error message: {}",
                id, err
            );
            log::error!("{}", err);
            return Err(err);
        }
    }
    log::info!("Inserted {} records of type CharacterStatDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_character_stats_state(ctx: &ReducerContext, records: Vec<CharacterStatsState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type CharacterStatsState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.character_stats_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type CharacterStatsState", len);
}

#[spacetimedb::reducer]
pub fn import_chat_message_state(ctx: &ReducerContext, records: Vec<ChatMessageState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ChatMessageState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.chat_message_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ChatMessageState", len);
}

#[spacetimedb::reducer]
pub fn import_chest_rarity_desc(ctx: &ReducerContext, records: Vec<ChestRarityDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_chest_rarity_desc_internal(ctx, records)?;
    Ok(())
}
fn import_chest_rarity_desc_internal(ctx: &ReducerContext, records: Vec<ChestRarityDesc>) -> Result<(), String> {
    for id in ctx.db.chest_rarity_desc().iter().map(|item| item.id) {
        ctx.db.chest_rarity_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ChestRarityDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.chest_rarity_desc().try_insert(record) {
            return Err(format!("Couldn't insert ChestRarityDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ChestRarityDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_claim_state(ctx: &ReducerContext, records: Vec<ClaimState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ClaimState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.claim_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ClaimState", len);
}

#[spacetimedb::reducer]
pub fn import_claim_local_state(ctx: &ReducerContext, records: Vec<ClaimLocalState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ClaimLocalState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.claim_local_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ClaimLocalState", len);
}

#[spacetimedb::reducer]
pub fn import_claim_recruitment_state(ctx: &ReducerContext, records: Vec<ClaimRecruitmentState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ClaimRecruitmentState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.claim_recruitment_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ClaimRecruitmentState", len);
}

#[spacetimedb::reducer]
pub fn import_claim_tech_desc(ctx: &ReducerContext, records: Vec<ClaimTechDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_claim_tech_desc_internal(ctx, records)?;
    Ok(())
}
fn import_claim_tech_desc_internal(ctx: &ReducerContext, records: Vec<ClaimTechDescV2>) -> Result<(), String> {
    for id in ctx.db.claim_tech_desc_v2().iter().map(|item| item.id) {
        ctx.db.claim_tech_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ClaimTechDescV2", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.claim_tech_desc_v2().try_insert(record) {
            return Err(format!("Couldn't insert ClaimTechDescV2 record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ClaimTechDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_claim_tech_state(ctx: &ReducerContext, records: Vec<ClaimTechState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ClaimTechState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.claim_tech_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ClaimTechState", len);
}

#[spacetimedb::reducer]
pub fn import_claim_tile_cost(ctx: &ReducerContext, records: Vec<ClaimTileCost>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_claim_tile_cost_internal(ctx, records)?;
    Ok(())
}
fn import_claim_tile_cost_internal(ctx: &ReducerContext, records: Vec<ClaimTileCost>) -> Result<(), String> {
    for id in ctx.db.claim_tile_cost().iter().map(|item| item.tile_count) {
        ctx.db.claim_tile_cost().tile_count().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ClaimTileCost", len);
    for record in records {
        let id = record.tile_count;
        if let Err(err) = ctx.db.claim_tile_cost().try_insert(record) {
            return Err(format!(
                "Couldn't insert ClaimTileCost record with tile_count {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ClaimTileCost", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_claim_tile_state(ctx: &ReducerContext, records: Vec<ClaimTileState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ClaimTileState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.claim_tile_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ClaimTileState", len);
}

#[spacetimedb::reducer]
pub fn import_climb_requirement_desc(ctx: &ReducerContext, records: Vec<ClimbRequirementDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_climb_requirement_desc_internal(ctx, records)?;
    Ok(())
}
fn import_climb_requirement_desc_internal(ctx: &ReducerContext, records: Vec<ClimbRequirementDesc>) -> Result<(), String> {
    for id in ctx.db.climb_requirement_desc().iter().map(|item| item.id) {
        ctx.db.climb_requirement_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ClimbRequirementDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.climb_requirement_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ClimbRequirementDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ClimbRequirementDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_clothing_desc(ctx: &ReducerContext, records: Vec<ClothingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_clothing_desc_internal(ctx, records)?;
    Ok(())
}
fn import_clothing_desc_internal(ctx: &ReducerContext, records: Vec<ClothingDesc>) -> Result<(), String> {
    for id in ctx.db.clothing_desc().iter().map(|item| item.item_id) {
        ctx.db.clothing_desc().item_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ClothingDesc", len);
    for record in records {
        let id = record.item_id;
        if let Err(err) = ctx.db.clothing_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ClothingDesc record with item_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ClothingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_collectible_desc(ctx: &ReducerContext, records: Vec<CollectibleDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_collectible_desc_internal(ctx, records)?;
    Ok(())
}
fn import_collectible_desc_internal(ctx: &ReducerContext, records: Vec<CollectibleDesc>) -> Result<(), String> {
    for id in ctx.db.collectible_desc().iter().map(|item| item.id) {
        ctx.db.collectible_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CollectibleDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.collectible_desc().try_insert(record) {
            return Err(format!("Couldn't insert CollectibleDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type CollectibleDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_combat_action_desc_v3(ctx: &ReducerContext, records: Vec<CombatActionDescV3>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_combat_action_desc_v3_internal(ctx, records)?;
    Ok(())
}
fn import_combat_action_desc_v3_internal(ctx: &ReducerContext, records: Vec<CombatActionDescV3>) -> Result<(), String> {
    for id in ctx.db.combat_action_desc_v3().iter().map(|item| item.id) {
        ctx.db.combat_action_desc_v3().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CombatActionDescV3", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.combat_action_desc_v3().try_insert(record) {
            return Err(format!(
                "Couldn't insert CombatActionDescV3 record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type CombatActionDescV3", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_combat_action_multi_hit_desc(ctx: &ReducerContext, records: Vec<CombatActionMultiHitDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_combat_action_multi_hit_desc_internal(ctx, records)?;
    Ok(())
}
fn import_combat_action_multi_hit_desc_internal(ctx: &ReducerContext, records: Vec<CombatActionMultiHitDesc>) -> Result<(), String> {
    for id in ctx.db.combat_action_multi_hit_desc().iter().map(|item| item.id) {
        ctx.db.combat_action_multi_hit_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CombatActionMultiHitDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.combat_action_multi_hit_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert CombatActionMultiHitDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type CombatActionMultiHitDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_combat_state(ctx: &ReducerContext, records: Vec<CombatState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type CombatState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.combat_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type CombatState", len);
}

#[spacetimedb::reducer]
pub fn import_config(ctx: &ReducerContext, records: Vec<Config>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type Config", records.len());
    let len = records.len();
    for record in records {
        ctx.db.config().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type Config", len);
}

#[spacetimedb::reducer]
pub fn import_construction_recipe_desc(ctx: &ReducerContext, records: Vec<ConstructionRecipeDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_construction_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_construction_recipe_desc_internal(ctx: &ReducerContext, records: Vec<ConstructionRecipeDescV2>) -> Result<(), String> {
    for id in ctx.db.construction_recipe_desc_v2().iter().map(|item| item.id) {
        ctx.db.construction_recipe_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ConstructionRecipeDescV2", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.construction_recipe_desc_v2().try_insert(record) {
            return Err(format!(
                "Couldn't insert ConstructionRecipeDescV2 record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ConstructionRecipeDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_crafting_recipe_desc(ctx: &ReducerContext, records: Vec<CraftingRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_crafting_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_crafting_recipe_desc_internal(ctx: &ReducerContext, records: Vec<CraftingRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.crafting_recipe_desc().iter().map(|item| item.id) {
        ctx.db.crafting_recipe_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type CraftingRecipeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.crafting_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert CraftingRecipeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type CraftingRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_deconstruction_recipe_desc(ctx: &ReducerContext, records: Vec<DeconstructionRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_deconstruction_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_deconstruction_recipe_desc_internal(ctx: &ReducerContext, records: Vec<DeconstructionRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.deconstruction_recipe_desc().iter().map(|item| item.id) {
        ctx.db.deconstruction_recipe_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type DeconstructionRecipeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.deconstruction_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert DeconstructionRecipeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type DeconstructionRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_dimension_description_state(ctx: &ReducerContext, records: Vec<DimensionDescriptionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type DimensionDescriptionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.dimension_description_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type DimensionDescriptionState", len);
}

#[spacetimedb::reducer]
pub fn import_dimension_network_description_state(ctx: &ReducerContext, records: Vec<DimensionNetworkState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type DimensionNetworkDescriptionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.dimension_network_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type DimensionNetworkDescriptionState", len);
}

#[spacetimedb::reducer]
pub fn import_emote_desc(ctx: &ReducerContext, records: Vec<EmoteDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_emote_desc_internal(ctx, records)?;
    Ok(())
}
fn import_emote_desc_internal(ctx: &ReducerContext, records: Vec<EmoteDescV2>) -> Result<(), String> {
    for id in ctx.db.emote_desc_v2().iter().map(|item| item.id) {
        ctx.db.emote_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmoteDescV2", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.emote_desc_v2().try_insert(record) {
            return Err(format!("Couldn't insert EmoteDescV2 record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type EmoteDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_enemy_ai_params_desc(ctx: &ReducerContext, records: Vec<EnemyAiParamsDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_enemy_ai_params_desc_internal(ctx, records)?;
    Ok(())
}
fn import_enemy_ai_params_desc_internal(ctx: &ReducerContext, records: Vec<EnemyAiParamsDesc>) -> Result<(), String> {
    for id in ctx.db.enemy_ai_params_desc().iter().map(|item| item.id) {
        ctx.db.enemy_ai_params_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EnemyAiParamsDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.enemy_ai_params_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EnemyAiParamsDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EnemyAiParamsDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_enemy_desc(ctx: &ReducerContext, records: Vec<EnemyDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_enemy_desc_internal(ctx, records)?;
    Ok(())
}
fn import_enemy_desc_internal(ctx: &ReducerContext, records: Vec<EnemyDesc>) -> Result<(), String> {
    for id in ctx.db.enemy_desc().iter().map(|item: EnemyDesc| item.enemy_type) {
        ctx.db.enemy_desc().enemy_type().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EnemyDesc", len);
    for record in records {
        let id = record.enemy_type;
        if let Err(err) = ctx.db.enemy_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EnemyDesc record with enemy_type {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type EnemyDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_enemy_state(ctx: &ReducerContext, records: Vec<EnemyState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type EnemyState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.enemy_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type EnemyState", len);
}

#[spacetimedb::reducer]
pub fn import_environment_debuff_desc(ctx: &ReducerContext, records: Vec<EnvironmentDebuffDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_environment_debuff_desc_internal(ctx, records)?;
    Ok(())
}
fn import_environment_debuff_desc_internal(ctx: &ReducerContext, records: Vec<EnvironmentDebuffDesc>) -> Result<(), String> {
    for id in ctx.db.environment_debuff_desc().iter().map(|item| item.buff_id) {
        ctx.db.environment_debuff_desc().buff_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EnvironmentDebuffDesc", len);
    for record in records {
        let id = record.buff_id;
        if let Err(err) = ctx.db.environment_debuff_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EnvironmentDebuffDesc record with buff_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EnvironmentDebuffDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_equipment_desc(ctx: &ReducerContext, records: Vec<EquipmentDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_equipment_desc_internal(ctx, records)?;
    Ok(())
}
fn import_equipment_desc_internal(ctx: &ReducerContext, records: Vec<EquipmentDesc>) -> Result<(), String> {
    for id in ctx.db.equipment_desc().iter().map(|item| item.item_id) {
        ctx.db.equipment_desc().item_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EquipmentDesc", len);
    for record in records {
        let id = record.item_id;
        if let Err(err) = ctx.db.equipment_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EquipmentDesc record with item_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EquipmentDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_equipment_state(ctx: &ReducerContext, records: Vec<EquipmentState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type EquipmentState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.equipment_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type EquipmentState", len);
}

#[spacetimedb::reducer]
pub fn import_experience_state(ctx: &ReducerContext, records: Vec<ExperienceState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ExperienceState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.experience_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ExperienceState", len);
}

#[spacetimedb::reducer]
pub fn import_exploration_chunks_state(ctx: &ReducerContext, records: Vec<ExplorationChunksState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ExplorationChunksState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.exploration_chunks_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ExplorationChunksState", len);
}

#[spacetimedb::reducer]
pub fn import_extraction_recipe_desc(ctx: &ReducerContext, records: Vec<ExtractionRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_extraction_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_extraction_recipe_desc_internal(ctx: &ReducerContext, records: Vec<ExtractionRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.extraction_recipe_desc().iter().map(|item| item.id) {
        ctx.db.extraction_recipe_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ExtractionRecipeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.extraction_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ExtractionRecipeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ExtractionRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_food_desc(ctx: &ReducerContext, records: Vec<FoodDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_food_desc_internal(ctx, records)?;
    Ok(())
}
fn import_food_desc_internal(ctx: &ReducerContext, records: Vec<FoodDesc>) -> Result<(), String> {
    for id in ctx.db.food_desc().iter().map(|item| item.item_id) {
        ctx.db.food_desc().item_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type FoodDesc", len);
    for record in records {
        let id = record.item_id;
        if let Err(err) = ctx.db.food_desc().try_insert(record) {
            return Err(format!("Couldn't insert FoodDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type FoodDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_footprint_tile_state(ctx: &ReducerContext, records: Vec<FootprintTileState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type FootprintTileState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.footprint_tile_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type FootprintTileState", len);
}

#[spacetimedb::reducer]
pub fn import_globals(ctx: &ReducerContext, records: Vec<Globals>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type Globals", records.len());
    let len = records.len();
    for record in records {
        ctx.db.globals().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type Globals", len);
}

#[spacetimedb::reducer]
pub fn import_world_region_state(ctx: &ReducerContext, records: Vec<WorldRegionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type WorldRegionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.world_region_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type WorldRegionState", len);
}

#[spacetimedb::reducer]
pub fn import_world_region_name_state(ctx: &ReducerContext, records: Vec<WorldRegionNameState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type WorldRegionNameState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.world_region_name_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type WorldRegionNameState", len);
}

#[spacetimedb::reducer]
pub fn import_growth_state(ctx: &ReducerContext, records: Vec<GrowthState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type GrowthState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.growth_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type GrowthState", len);
}

#[spacetimedb::reducer]
pub fn import_health_state(ctx: &ReducerContext, records: Vec<HealthState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type HealthState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.health_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type HealthState", len);
}

#[spacetimedb::reducer]
pub fn import_herd_cache(ctx: &ReducerContext, records: Vec<HerdState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type HerdCache", records.len());
    let len = records.len();
    for record in records {
        ctx.db.herd_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type HerdCache", len);
}

#[spacetimedb::reducer]
pub fn import_satiation_state(ctx: &ReducerContext, records: Vec<SatiationState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type SatiationState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.satiation_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type SatiationState", len);
}

#[spacetimedb::reducer]
pub fn import_identity_role(ctx: &ReducerContext, records: Vec<IdentityRole>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type IdentityRole", records.len());
    let len = records.len();
    for record in records {
        ctx.db.identity_role().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type IdentityRole", len);
}

#[spacetimedb::reducer]
pub fn import_interior_collapse_trigger_state(ctx: &ReducerContext, records: Vec<InteriorCollapseTriggerState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type InteriorCollapseTriggerState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.interior_collapse_trigger_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type InteriorCollapseTriggerState", len);
}

#[spacetimedb::reducer]
pub fn import_interior_instance_desc(ctx: &ReducerContext, records: Vec<InteriorInstanceDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_instance_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_instance_desc_internal(ctx: &ReducerContext, records: Vec<InteriorInstanceDesc>) -> Result<(), String> {
    for id in ctx.db.interior_instance_desc().iter().map(|item| item.id) {
        ctx.db.interior_instance_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorInstanceDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.interior_instance_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorInstanceDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorInstanceDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_interior_environment_desc(ctx: &ReducerContext, records: Vec<InteriorEnvironmentDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_environment_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_environment_desc_internal(ctx: &ReducerContext, records: Vec<InteriorEnvironmentDesc>) -> Result<(), String> {
    for id in ctx.db.interior_environment_desc().iter().map(|item| item.id) {
        ctx.db.interior_environment_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorEnvironmentDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.interior_environment_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorEnvironmentDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorEnvironmentDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_interior_network_desc(ctx: &ReducerContext, records: Vec<InteriorNetworkDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_network_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_network_desc_internal(ctx: &ReducerContext, records: Vec<InteriorNetworkDesc>) -> Result<(), String> {
    for id in ctx.db.interior_network_desc().iter().map(|item| item.building_id) {
        ctx.db.interior_network_desc().building_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorNetworkDesc", len);
    for record in records {
        let id = record.building_id;
        if let Err(err) = ctx.db.interior_network_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorNetworkDesc record with building_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorNetworkDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_interior_portal_connections_desc(ctx: &ReducerContext, records: Vec<InteriorPortalConnectionsDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_portal_connections_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_portal_connections_desc_internal(
    ctx: &ReducerContext,
    records: Vec<InteriorPortalConnectionsDesc>,
) -> Result<(), String> {
    for id in ctx.db.interior_portal_connections_desc().iter().map(|item| item.id) {
        ctx.db.interior_portal_connections_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorPortalConnectionsDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.interior_portal_connections_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorPortalConnectionsDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorPortalConnectionsDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_interior_shape_desc(ctx: &ReducerContext, records: Vec<InteriorShapeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_shape_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_shape_desc_internal(ctx: &ReducerContext, records: Vec<InteriorShapeDesc>) -> Result<(), String> {
    for id in ctx.db.interior_shape_desc().iter().map(|item| item.id) {
        ctx.db.interior_shape_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorShapeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.interior_shape_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorShapeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorShapeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_interior_spawn_desc(ctx: &ReducerContext, records: Vec<InteriorSpawnDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_interior_spawn_desc_internal(ctx, records)?;
    Ok(())
}
fn import_interior_spawn_desc_internal(ctx: &ReducerContext, records: Vec<InteriorSpawnDesc>) -> Result<(), String> {
    for id in ctx.db.interior_spawn_desc().iter().map(|item| item.id) {
        ctx.db.interior_spawn_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type InteriorSpawnDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.interior_spawn_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert InteriorSpawnDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type InteriorSpawnDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_inventory_state(ctx: &ReducerContext, records: Vec<InventoryState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type InventoryState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.inventory_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type InventoryState", len);
}

#[spacetimedb::reducer]
pub fn import_item_conversion_recipe_desc(ctx: &ReducerContext, records: Vec<ItemConversionRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_item_conversion_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_item_conversion_recipe_desc_internal(ctx: &ReducerContext, records: Vec<ItemConversionRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.item_conversion_recipe_desc().iter().map(|item| item.id) {
        ctx.db.item_conversion_recipe_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ItemConversionRecipeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.item_conversion_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ItemConversionRecipeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ItemConversionRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_item_desc(ctx: &ReducerContext, records: Vec<ItemDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_item_desc_internal(ctx, records)?;
    Ok(())
}
fn import_item_desc_internal(ctx: &ReducerContext, records: Vec<ItemDesc>) -> Result<(), String> {
    for id in ctx.db.item_desc().iter().map(|item| item.id) {
        ctx.db.item_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ItemDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.item_desc().try_insert(record) {
            return Err(format!("Couldn't insert ItemDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ItemDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_item_list_desc(ctx: &ReducerContext, records: Vec<ItemListDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_item_list_desc_internal(ctx, records)?;
    Ok(())
}
fn import_item_list_desc_internal(ctx: &ReducerContext, records: Vec<ItemListDesc>) -> Result<(), String> {
    for id in ctx.db.item_list_desc().iter().map(|item| item.id) {
        ctx.db.item_list_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ItemListDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.item_list_desc().try_insert(record) {
            return Err(format!("Couldn't insert ItemListDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ItemListDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_dropped_inventory_state(ctx: &ReducerContext, records: Vec<DroppedInventoryState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type DroppedInventoryState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.dropped_inventory_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type DroppedInventoryState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_achievement_state(ctx: &ReducerContext, records: Vec<KnowledgeAchievementState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeAchievementState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_achievement_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeAchievementState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_battle_action_state(ctx: &ReducerContext, records: Vec<KnowledgeBattleActionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeBattleActionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_battle_action_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeBattleActionState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_building_state(ctx: &ReducerContext, records: Vec<KnowledgeBuildingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeBuildingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_building_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeBuildingState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_cargo_state(ctx: &ReducerContext, records: Vec<KnowledgeCargoState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeCargoState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_cargo_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeCargoState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_construction_state(ctx: &ReducerContext, records: Vec<KnowledgeConstructionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeConstructionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_construction_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeConstructionState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_craft_state(ctx: &ReducerContext, records: Vec<KnowledgeCraftState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeCraftState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_craft_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeCraftState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_enemy_state(ctx: &ReducerContext, records: Vec<KnowledgeEnemyState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeEnemyState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_enemy_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeEnemyState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_extract_state(ctx: &ReducerContext, records: Vec<KnowledgeExtractState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeExtractState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_extract_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeExtractState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_item_state(ctx: &ReducerContext, records: Vec<KnowledgeItemState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeItemState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_item_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeItemState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_lore_state(ctx: &ReducerContext, records: Vec<KnowledgeLoreState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeLoreState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_lore_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeLoreState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_npc_state(ctx: &ReducerContext, records: Vec<KnowledgeNpcState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeNpcState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_npc_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeNpcState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_pillar_shaping_state(ctx: &ReducerContext, records: Vec<KnowledgePillarShapingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgePillarShapingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_pillar_shaping_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgePillarShapingState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_paving_state(ctx: &ReducerContext, records: Vec<KnowledgePavingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgePavingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_paving_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgePavingState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_resource_placement_state(ctx: &ReducerContext, records: Vec<KnowledgeResourcePlacementState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeResourcePlacementState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_resource_placement_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeResourcePlacementState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_resource_state(ctx: &ReducerContext, records: Vec<KnowledgeResourceState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeResourceState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_resource_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeResourceState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_ruins_state(ctx: &ReducerContext, records: Vec<KnowledgeRuinsState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeRuinsState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_ruins_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeRuinsState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_scroll_desc(ctx: &ReducerContext, records: Vec<KnowledgeScrollDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_knowledge_scroll_desc_internal(ctx, records)?;
    Ok(())
}
fn import_knowledge_scroll_desc_internal(ctx: &ReducerContext, records: Vec<KnowledgeScrollDesc>) -> Result<(), String> {
    for id in ctx.db.knowledge_scroll_desc().iter().map(|item| item.item_id) {
        ctx.db.knowledge_scroll_desc().item_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type KnowledgeScrollDesc", len);
    for record in records {
        let id = record.item_id;
        if let Err(err) = ctx.db.knowledge_scroll_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert KnowledgeScrollDesc record with item_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type KnowledgeScrollDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_knowledge_scroll_type_desc(ctx: &ReducerContext, records: Vec<KnowledgeScrollTypeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_knowledge_scroll_type_desc_internal(ctx, records)?;
    Ok(())
}
fn import_knowledge_scroll_type_desc_internal(ctx: &ReducerContext, records: Vec<KnowledgeScrollTypeDesc>) -> Result<(), String> {
    for id in ctx.db.knowledge_scroll_type_desc().iter().map(|item| item.id) {
        ctx.db.knowledge_scroll_type_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type KnowledgeScrollTypeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.knowledge_scroll_type_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert KnowledgeScrollTypeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type KnowledgeScrollTypeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_knowledge_secondary_state(ctx: &ReducerContext, records: Vec<KnowledgeSecondaryState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeSecondaryState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_secondary_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeSecondaryState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_vault_state(ctx: &ReducerContext, records: Vec<KnowledgeVaultState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeVaultState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_vault_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeVaultState", len);
}

#[spacetimedb::reducer]
pub fn import_knowledge_deployable_state(ctx: &ReducerContext, records: Vec<KnowledgeDeployableState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type KnowledgeDeployableState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.knowledge_deployable_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type KnowledgeDeployableState", len);
}

#[spacetimedb::reducer]
pub fn import_location_cache(ctx: &ReducerContext, records: Vec<LocationCache>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type LocationCache", records.len());
    let len = records.len();
    for record in records {
        ctx.db.location_cache().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type LocationCache", len);
}

#[spacetimedb::reducer]
pub fn import_location_state(ctx: &ReducerContext, records: Vec<LocationState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type LocationState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.location_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type LocationState", len);
}

#[spacetimedb::reducer]
pub fn import_loot_chest_desc(ctx: &ReducerContext, records: Vec<LootChestDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_loot_chest_desc_internal(ctx, records)?;
    Ok(())
}
fn import_loot_chest_desc_internal(ctx: &ReducerContext, records: Vec<LootChestDesc>) -> Result<(), String> {
    for id in ctx.db.loot_chest_desc().iter().map(|item| item.id) {
        ctx.db.loot_chest_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type LootChestDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.loot_chest_desc().try_insert(record) {
            return Err(format!("Couldn't insert LootChestDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type LootChestDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_loot_chest_state(ctx: &ReducerContext, records: Vec<LootChestState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type LootChestState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.loot_chest_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type LootChestState", len);
}

#[spacetimedb::reducer]
pub fn import_loot_rarity_desc(ctx: &ReducerContext, records: Vec<LootRarityDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_loot_rarity_desc_internal(ctx, records)?;
    Ok(())
}
fn import_loot_rarity_desc_internal(ctx: &ReducerContext, records: Vec<LootRarityDesc>) -> Result<(), String> {
    for id in ctx.db.loot_rarity_desc().iter().map(|item| item.id) {
        ctx.db.loot_rarity_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type LootRarityDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.loot_rarity_desc().try_insert(record) {
            return Err(format!("Couldn't insert LootRarityDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type LootRarityDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_loot_table_desc(ctx: &ReducerContext, records: Vec<LootTableDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_loot_table_desc_internal(ctx, records)?;
    Ok(())
}
fn import_loot_table_desc_internal(ctx: &ReducerContext, records: Vec<LootTableDesc>) -> Result<(), String> {
    for id in ctx.db.loot_table_desc().iter().map(|item| item.id) {
        ctx.db.loot_table_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type LootTableDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.loot_table_desc().try_insert(record) {
            return Err(format!("Couldn't insert LootTableDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type LootTableDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_mobile_entity_state(ctx: &ReducerContext, records: Vec<MobileEntityState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type MobileEntityState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.mobile_entity_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type MobileEntityState", len);
}

#[spacetimedb::reducer]
pub fn import_mounting_state(ctx: &ReducerContext, records: Vec<MountingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type MountingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.mounting_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type MountingState", len);
}

#[spacetimedb::reducer]
pub fn import_npc_desc(ctx: &ReducerContext, records: Vec<NpcDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_npc_desc_internal(ctx, records)?;
    Ok(())
}
fn import_npc_desc_internal(ctx: &ReducerContext, records: Vec<NpcDesc>) -> Result<(), String> {
    for id in ctx.db.npc_desc().iter().map(|item| item.npc_type) {
        ctx.db.npc_desc().npc_type().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type NpcDesc", len);
    for record in records {
        let id = record.npc_type;
        if let Err(err) = ctx.db.npc_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert NpcDesc record with npc_type {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type NpcDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_npc_state(ctx: &ReducerContext, records: Vec<NpcState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type NpcState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.npc_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type NpcState", len);
}

#[spacetimedb::reducer]
pub fn import_onboarding_reward_desc(ctx: &ReducerContext, records: Vec<OnboardingRewardDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_onboarding_reward_desc_internal(ctx, records)?;
    Ok(())
}
fn import_onboarding_reward_desc_internal(ctx: &ReducerContext, records: Vec<OnboardingRewardDesc>) -> Result<(), String> {
    for id in ctx.db.onboarding_reward_desc().iter().map(|item| item.state_id) {
        ctx.db.onboarding_reward_desc().state_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type OnboardingRewardDesc", len);
    for record in records {
        let id = record.state_id;
        if let Err(err) = ctx.db.onboarding_reward_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert OnboardingRewardDesc record with state_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type OnboardingRewardDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_onboarding_state(ctx: &ReducerContext, records: Vec<OnboardingState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type OnboardingState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.onboarding_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type OnboardingState", len);
}

#[spacetimedb::reducer]
pub fn import_parameters_desc(ctx: &ReducerContext, records: Vec<ParametersDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_parameters_desc_internal(ctx, records)?;
    Ok(())
}
fn import_parameters_desc_internal(ctx: &ReducerContext, records: Vec<ParametersDescV2>) -> Result<(), String> {
    for id in ctx.db.parameters_desc_v2().iter().map(|item| item.version) {
        ctx.db.parameters_desc_v2().version().delete(&id);
    }
    for id in ctx.db.parameters_player_move_desc().iter().map(|item| item.version) {
        ctx.db.parameters_player_move_desc().version().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ParametersDescV2", len);
    for record in records {
        let id = record.version;
        let move_params = ParametersPlayerMoveDesc {
            version: id,
            default_speed: record.default_speed.clone(),
        };
        if let Err(err) = ctx.db.parameters_player_move_desc().try_insert(move_params) {
            return Err(format!(
                "Couldn't insert ParametersPlayerMoveDesc record with version {id}. Error message: {err}"
            ));
        }
        if let Err(err) = ctx.db.parameters_desc_v2().try_insert(record) {
            return Err(format!(
                "Couldn't insert ParametersDescV2 record with version {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ParametersDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_private_parameters_desc(ctx: &ReducerContext, records: Vec<PrivateParametersDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_private_parameters_desc_internal(ctx, records)
}
fn import_private_parameters_desc_internal(ctx: &ReducerContext, records: Vec<PrivateParametersDesc>) -> Result<(), String> {
    for id in ctx.db.private_parameters_desc().iter().map(|item| item.version) {
        ctx.db.private_parameters_desc().version().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PrivateParametersDesc", len);
    for record in records {
        let id = record.version;
        if let Err(err) = ctx.db.private_parameters_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PrivateParametersDesc record with version {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type PrivateParametersDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_paved_tile_state(ctx: &ReducerContext, records: Vec<PavedTileState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type PavedTileState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.paved_tile_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type PavedTileState", len);
}

#[spacetimedb::reducer]
pub fn import_pathfinding_desc(ctx: &ReducerContext, records: Vec<PathfindingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_pathfinding_desc_internal(ctx, records)
}

pub fn import_pathfinding_desc_internal(ctx: &ReducerContext, records: Vec<PathfindingDesc>) -> Result<(), String> {
    for id in ctx.db.pathfinding_desc().iter().map(|item| item.id) {
        ctx.db.pathfinding_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PathfindingDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.pathfinding_desc().try_insert(record) {
            log::error!("Couldn't insert PathfindingDesc record with id {id}. Error message: {err}");
            return Err(format!("Couldn't insert PathfindingDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type PathfindingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_pillar_shaping_desc(ctx: &ReducerContext, records: Vec<PillarShapingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_pillar_shaping_desc_internal(ctx, records)?;
    Ok(())
}

fn import_pillar_shaping_desc_internal(ctx: &ReducerContext, records: Vec<PillarShapingDesc>) -> Result<(), String> {
    for id in ctx.db.pillar_shaping_desc().iter().map(|item| item.id) {
        ctx.db.pillar_shaping_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PillarShapingDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.pillar_shaping_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PillarShapingDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type PillarShapingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_paving_tile_desc(ctx: &ReducerContext, records: Vec<PavingTileDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_paving_tile_desc_internal(ctx, records)?;
    Ok(())
}
fn import_paving_tile_desc_internal(ctx: &ReducerContext, records: Vec<PavingTileDesc>) -> Result<(), String> {
    for id in ctx.db.paving_tile_desc().iter().map(|item| item.id) {
        ctx.db.paving_tile_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PavingTileDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.paving_tile_desc().try_insert(record) {
            return Err(format!("Couldn't insert PavingTileDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type PavingTileDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_player_action_state(ctx: &ReducerContext, records: Vec<PlayerActionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type PlayerActionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.player_action_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type PlayerActionState", len);
}

#[spacetimedb::reducer]
pub fn import_player_prefs_state(ctx: &ReducerContext, records: Vec<PlayerPrefsState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type PlayerPrefsState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.player_prefs_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type PlayerPrefsState", len);
}

#[spacetimedb::reducer]
pub fn import_player_state(ctx: &ReducerContext, records: Vec<PlayerState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type PlayerState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.player_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type PlayerState", len);
}

#[spacetimedb::reducer]
pub fn import_portal_state(ctx: &ReducerContext, records: Vec<PortalState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type PortalState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.portal_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type PortalState", len);
}

#[spacetimedb::reducer]
pub fn import_progressive_action_state(ctx: &ReducerContext, records: Vec<ProgressiveActionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ProgressiveActionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.progressive_action_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ProgressiveActionState", len);
}

#[spacetimedb::reducer]
pub fn import_project_site_state(ctx: &ReducerContext, records: Vec<ProjectSiteState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ProjectSiteState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.project_site_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ProjectSiteState", len);
}

#[spacetimedb::reducer]
pub fn import_rent_state(ctx: &ReducerContext, records: Vec<RentState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type RentState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.rent_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type RentState", len);
}

#[spacetimedb::reducer]
pub fn import_resource_clump_desc(ctx: &ReducerContext, records: Vec<ResourceClumpDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_resource_clump_desc_internal(ctx, records)?;
    Ok(())
}

fn import_resource_clump_desc_internal(ctx: &ReducerContext, records: Vec<ResourceClumpDesc>) -> Result<(), String> {
    for id in ctx.db.resource_clump_desc().iter().map(|item| item.id) {
        ctx.db.resource_clump_desc().id().delete(&id);
    }
    for id in ctx.db.single_resource_to_clump_desc().iter().map(|item| item.resource_id) {
        ctx.db.single_resource_to_clump_desc().resource_id().delete(&id);
    }

    let len: usize = records.len();
    log::info!("Will insert {} records of type ResourceClumpDesc", len);
    for record in records {
        let id = record.id;
        if record.resource_id.len() == 1 {
            if let Err(err) = ctx.db.single_resource_to_clump_desc().try_insert(SingleResourceToClumpDesc {
                resource_id: record.resource_id[0],
                clump_id: id,
            }) {
                let res_id = record.resource_id[0];
                let err_msg = format!("Ignoring duplicate SingleResourceClumpDesc record with resource_id {res_id}, clump_id: {id}: {err}");
                log::warn!("{}", err_msg);
            }
        }
        if let Err(err) = ctx.db.resource_clump_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ResourceClumpDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ResourceClumpDesc", len);

    Ok(())
}

#[spacetimedb::reducer]
pub fn import_resource_count(ctx: &ReducerContext, records: Vec<ResourceCount>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ResourceCount", records.len());
    let len = records.len();
    for record in records {
        ctx.db.resource_count().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ResourceCount", len);
}

#[spacetimedb::reducer]
pub fn import_resource_desc(ctx: &ReducerContext, records: Vec<ResourceDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_resource_desc_internal(ctx, records)?;
    Ok(())
}
fn import_resource_desc_internal(ctx: &ReducerContext, records: Vec<ResourceDesc>) -> Result<(), String> {
    for id in ctx.db.resource_desc().iter().map(|item| item.id) {
        ctx.db.resource_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ResourceDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.resource_desc().try_insert(record) {
            return Err(format!("Couldn't insert ResourceDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ResourceDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_resource_growth_recipe_desc(ctx: &ReducerContext, records: Vec<ResourceGrowthRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_resource_growth_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_resource_growth_recipe_desc_internal(ctx: &ReducerContext, records: Vec<ResourceGrowthRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.resource_growth_recipe_desc().iter().map(|item| item.id) {
        ctx.db.resource_growth_recipe_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ResourceGrowthRecipeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.resource_growth_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ResourceGrowthRecipeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ResourceGrowthRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_resource_placement_recipe_desc(ctx: &ReducerContext, records: Vec<ResourcePlacementRecipeDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_resource_placement_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_resource_placement_recipe_desc_internal(ctx: &ReducerContext, records: Vec<ResourcePlacementRecipeDescV2>) -> Result<(), String> {
    for id in ctx.db.resource_placement_recipe_desc_v2().iter().map(|item| item.id) {
        ctx.db.resource_placement_recipe_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ResourcePlacementRecipeDescV2", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.resource_placement_recipe_desc_v2().try_insert(record) {
            return Err(format!(
                "Couldn't insert ResourcePlacementRecipeDescV2 record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type ResourcePlacementRecipeDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_resource_state(ctx: &ReducerContext, records: Vec<ResourceState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ResourceState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.resource_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ResourceState", len);
}

#[spacetimedb::reducer]
pub fn import_resources_log(ctx: &ReducerContext, records: Vec<ResourcesLog>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ResourcesLog", records.len());
    let len = records.len();
    for record in records {
        ctx.db.resources_log().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ResourcesLog", len);
}

#[spacetimedb::reducer]
pub fn import_secondary_knowledge_desc(ctx: &ReducerContext, records: Vec<SecondaryKnowledgeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_secondary_knowledge_desc_internal(ctx, records)?;
    Ok(())
}
fn import_secondary_knowledge_desc_internal(ctx: &ReducerContext, records: Vec<SecondaryKnowledgeDesc>) -> Result<(), String> {
    for id in ctx.db.secondary_knowledge_desc().iter().map(|item| item.id) {
        ctx.db.secondary_knowledge_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type SecondaryKnowledgeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.secondary_knowledge_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert SecondaryKnowledgeDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type SecondaryKnowledgeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_server_identity(ctx: &ReducerContext, records: Vec<ServerIdentity>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type ServerIdentity", records.len());
    let len = records.len();
    for record in records {
        ctx.db.server_identity().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type ServerIdentity", len);
}

#[spacetimedb::reducer]
pub fn import_signed_in_player_state(ctx: &ReducerContext, records: Vec<SignedInPlayerState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type SignedInPlayerState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.signed_in_player_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type SignedInPlayerState", len);
}

#[spacetimedb::reducer]
pub fn import_stamina_state(ctx: &ReducerContext, records: Vec<StaminaState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type StaminaState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.stamina_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type StaminaState", len);
}

#[spacetimedb::reducer]
pub fn import_target_state(ctx: &ReducerContext, records: Vec<TargetState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type TargetState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.target_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type TargetState", len);
}

#[spacetimedb::reducer]
pub fn import_targetable_state(ctx: &ReducerContext, records: Vec<TargetableState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type TargetableState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.targetable_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type TargetableState", len);
}

#[spacetimedb::reducer]
pub fn import_targeting_matrix_desc(ctx: &ReducerContext, records: Vec<TargetingMatrixDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_targeting_matrix_desc_internal(ctx, records)?;
    Ok(())
}
fn import_targeting_matrix_desc_internal(ctx: &ReducerContext, records: Vec<TargetingMatrixDesc>) -> Result<(), String> {
    for id in ctx.db.targeting_matrix_desc().iter().map(|item| item.id) {
        ctx.db.targeting_matrix_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type TargetingMatrixDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.targeting_matrix_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert TargetingMatrixDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type TargetingMatrixDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_teleport_item_desc(ctx: &ReducerContext, records: Vec<TeleportItemDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_teleport_item_desc_internal(ctx, records)?;
    Ok(())
}
fn import_teleport_item_desc_internal(ctx: &ReducerContext, records: Vec<TeleportItemDesc>) -> Result<(), String> {
    for id in ctx.db.teleport_item_desc().iter().map(|item| item.id) {
        ctx.db.teleport_item_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type TeleportItemDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.teleport_item_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert TeleportItemDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type TeleportItemDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_terraform_recipe_desc(ctx: &ReducerContext, records: Vec<TerraformRecipeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_terraform_recipe_desc_internal(ctx, records)?;
    Ok(())
}
fn import_terraform_recipe_desc_internal(ctx: &ReducerContext, records: Vec<TerraformRecipeDesc>) -> Result<(), String> {
    for id in ctx.db.terraform_recipe_desc().iter().map(|item| item.difference) {
        ctx.db.terraform_recipe_desc().difference().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type TerraformRecipeDesc", len);
    for record in records {
        let id = record.difference;
        if let Err(err) = ctx.db.terraform_recipe_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert TerraformRecipeDesc record with difference {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type TerraformRecipeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_terrain_chunk_state(ctx: &ReducerContext, records: Vec<TerrainChunkState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type TerrainChunkState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.terrain_chunk_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type TerrainChunkState", len);
}

#[spacetimedb::reducer]
pub fn import_tool_desc(ctx: &ReducerContext, records: Vec<ToolDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_tool_desc_internal(ctx, records)?;
    Ok(())
}
fn import_tool_desc_internal(ctx: &ReducerContext, records: Vec<ToolDesc>) -> Result<(), String> {
    for id in ctx.db.tool_desc().iter().map(|item| item.id) {
        ctx.db.tool_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ToolDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.tool_desc().try_insert(record) {
            return Err(format!("Couldn't insert ToolDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ToolDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_tool_type_desc(ctx: &ReducerContext, records: Vec<ToolTypeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_tool_type_desc_internal(ctx, records)?;
    Ok(())
}
fn import_tool_type_desc_internal(ctx: &ReducerContext, records: Vec<ToolTypeDesc>) -> Result<(), String> {
    for id in ctx.db.tool_type_desc().iter().map(|item| item.id) {
        ctx.db.tool_type_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ToolTypeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.tool_type_desc().try_insert(record) {
            return Err(format!("Couldn't insert ToolTypeDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ToolTypeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_trade_order_state(ctx: &ReducerContext, records: Vec<TradeOrderState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type TradeOrderState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.trade_order_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type TradeOrderState", len);
}

#[spacetimedb::reducer]
pub fn import_trade_session_state(ctx: &ReducerContext, records: Vec<TradeSessionState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type TradeSessionState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.trade_session_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type TradeSessionState", len);
}

#[spacetimedb::reducer]
pub fn import_traveler_trade_order_desc(ctx: &ReducerContext, records: Vec<TravelerTradeOrderDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_traveler_trade_order_desc_internal(ctx, records)?;
    Ok(())
}
fn import_traveler_trade_order_desc_internal(ctx: &ReducerContext, records: Vec<TravelerTradeOrderDesc>) -> Result<(), String> {
    for id in ctx.db.traveler_trade_order_desc().iter().map(|item| item.id) {
        ctx.db.traveler_trade_order_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type TravelerTradeOrderDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.traveler_trade_order_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert TravelerTradeOrderDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type TravelerTradeOrderDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_traveler_task_desc(ctx: &ReducerContext, records: Vec<TravelerTaskDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_traveler_task_desc_internal(ctx, records)?;
    Ok(())
}
fn import_traveler_task_desc_internal(ctx: &ReducerContext, records: Vec<TravelerTaskDesc>) -> Result<(), String> {
    for id in ctx.db.traveler_task_desc().iter().map(|item| item.id) {
        ctx.db.traveler_task_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type TravelerTaskDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.traveler_task_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert TravelerTaskDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type TravelerTaskDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_user_moderation_state(ctx: &ReducerContext, records: Vec<UserModerationState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type UserModerationState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.user_moderation_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type UserModerationState", len);
}

#[spacetimedb::reducer]
pub fn import_user_state(ctx: &ReducerContext, records: Vec<UserState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type UserState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.user_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type UserState", len);
}

#[spacetimedb::reducer]
pub fn import_vault_state(ctx: &ReducerContext, records: Vec<VaultState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type VaultState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.vault_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type VaultState", len);
}

#[spacetimedb::reducer]
pub fn import_deployable_desc(ctx: &ReducerContext, records: Vec<DeployableDescV4>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_deployable_desc_internal(ctx, records)?;
    Ok(())
}
fn import_deployable_desc_internal(ctx: &ReducerContext, records: Vec<DeployableDescV4>) -> Result<(), String> {
    for id in ctx.db.deployable_desc_v4().iter().map(|item| item.id) {
        ctx.db.deployable_desc_v4().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type DeployableDescV4", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.deployable_desc_v4().try_insert(record) {
            return Err(format!(
                "Couldn't insert DeployableDescV4 record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type DeployableDescV4", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_deployable_state(ctx: &ReducerContext, records: Vec<DeployableState>) {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        log::error!("Invalid permissions");
        return ();
    }
    log::info!("Will insert {} records of type DeployableState", records.len());
    let len = records.len();
    for record in records {
        ctx.db.deployable_state().try_insert(record).unwrap();
    }
    log::info!("Inserted {} records of type DeployableState", len);
}

#[spacetimedb::reducer]
pub fn import_weapon_desc(ctx: &ReducerContext, records: Vec<WeaponDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_weapon_desc_internal(ctx, records)?;
    Ok(())
}
fn import_weapon_desc_internal(ctx: &ReducerContext, records: Vec<WeaponDesc>) -> Result<(), String> {
    for id in ctx.db.weapon_desc().iter().map(|item| item.item_id) {
        ctx.db.weapon_desc().item_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type WeaponDesc", len);
    for record in records {
        let id = record.item_id;
        if let Err(err) = ctx.db.weapon_desc().try_insert(record) {
            return Err(format!("Couldn't insert WeaponDesc record with item_id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type WeaponDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_weapon_type_desc(ctx: &ReducerContext, records: Vec<WeaponTypeDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_weapon_type_desc_internal(ctx, records)?;
    Ok(())
}
fn import_weapon_type_desc_internal(ctx: &ReducerContext, records: Vec<WeaponTypeDesc>) -> Result<(), String> {
    for id in ctx.db.weapon_type_desc().iter().map(|item| item.id) {
        ctx.db.weapon_type_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type WeaponTypeDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.weapon_type_desc().try_insert(record) {
            return Err(format!("Couldn't insert WeaponTypeDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type WeaponTypeDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_rank_desc(ctx: &ReducerContext, records: Vec<EmpireRankDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_rank_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_rank_desc_internal(ctx: &ReducerContext, records: Vec<EmpireRankDesc>) -> Result<(), String> {
    for id in ctx.db.empire_rank_desc().iter().map(|item| item.rank) {
        ctx.db.empire_rank_desc().rank().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireRankDesc", len);
    for record in records {
        let id = record.rank;
        if let Err(err) = ctx.db.empire_rank_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EmpireRankDesc record with rank {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EmpireRankDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_supplies_desc(ctx: &ReducerContext, records: Vec<EmpireSuppliesDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_supplies_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_supplies_desc_internal(ctx: &ReducerContext, records: Vec<EmpireSuppliesDesc>) -> Result<(), String> {
    for id in ctx.db.empire_supplies_desc().iter().map(|item| item.cargo_id) {
        ctx.db.empire_supplies_desc().cargo_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireSuppliesDesc", len);
    for record in records {
        let id = record.cargo_id;
        if let Err(err) = ctx.db.empire_supplies_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EmpireSuppliesDesc record with cargo_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EmpireSuppliesDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_notification_desc(ctx: &ReducerContext, records: Vec<EmpireNotificationDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_notification_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_notification_desc_internal(ctx: &ReducerContext, records: Vec<EmpireNotificationDesc>) -> Result<(), String> {
    for id in ctx.db.empire_notification_desc().iter().map(|item| item.id) {
        ctx.db.empire_notification_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireNotificationDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.empire_notification_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EmpireNotificationDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EmpireNotificationDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_territory_desc(ctx: &ReducerContext, records: Vec<EmpireTerritoryDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_territory_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_territory_desc_internal(ctx: &ReducerContext, records: Vec<EmpireTerritoryDesc>) -> Result<(), String> {
    for id in ctx.db.empire_territory_desc().iter().map(|item| item.id) {
        ctx.db.empire_territory_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireTerritoryDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.empire_territory_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EmpireTerritoryDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type EmpireTerritoryDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_colors_desc(ctx: &ReducerContext, records: Vec<EmpireColorDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_colors_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_colors_desc_internal(ctx: &ReducerContext, records: Vec<EmpireColorDesc>) -> Result<(), String> {
    for id in ctx.db.empire_color_desc().iter().map(|item| item.id) {
        ctx.db.empire_color_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireColorDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.empire_color_desc().try_insert(record) {
            return Err(format!("Couldn't insert EmpireColorDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type EmpireColorDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_empire_icon_desc(ctx: &ReducerContext, records: Vec<EmpireIconDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_empire_icon_desc_internal(ctx, records)?;
    Ok(())
}
fn import_empire_icon_desc_internal(ctx: &ReducerContext, records: Vec<EmpireIconDesc>) -> Result<(), String> {
    for id in ctx.db.empire_icon_desc().iter().map(|item| item.id) {
        ctx.db.empire_icon_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EmpireIconDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.empire_icon_desc().try_insert(record) {
            return Err(format!("Couldn't insert EmpireIconDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type EmpireIconDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_wall_desc(ctx: &ReducerContext, records: Vec<WallDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_wall_desc_internal(ctx, records)?;
    Ok(())
}
fn import_wall_desc_internal(ctx: &ReducerContext, records: Vec<WallDesc>) -> Result<(), String> {
    for id in ctx.db.wall_desc().iter().map(|item| item.building_id) {
        ctx.db.wall_desc().building_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type WallDesc", len);
    for record in records {
        let id = record.building_id;
        if let Err(err) = ctx.db.wall_desc().try_insert(record) {
            return Err(format!("Couldn't insert WallDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type WallDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_gate_desc(ctx: &ReducerContext, records: Vec<GateDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_gate_desc_internal(ctx, records)?;
    Ok(())
}
fn import_gate_desc_internal(ctx: &ReducerContext, records: Vec<GateDesc>) -> Result<(), String> {
    for id in ctx.db.gate_desc().iter().map(|item| item.building_id) {
        ctx.db.gate_desc().building_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type WallDesc", len);
    for record in records {
        let id = record.building_id;
        if let Err(err) = ctx.db.gate_desc().try_insert(record) {
            return Err(format!("Couldn't insert WallDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type WallDesc", len);
    Ok(())
}

fn import_static_data_post_processing(ctx: &ReducerContext) -> Result<(), String> {
    //create ItemConversions for Items with item lists
    for item in ctx.db.item_desc().iter() {
        let item_id = item.id;
        let item_list_id = item.item_list_id;

        if item_list_id > 0 {
            let item_stack = ItemStack {
                item_id: item_id,
                quantity: 1,
                item_type: ItemType::Item,
                durability: None,
            };

            let item_list_conversion = ItemConversionRecipeDesc {
                id: item_id,
                name: format!("Resolve Item {}", item_id),
                time_cost: 1,
                stamina_cost: 0,
                location_context: 0,
                string_context: "Collect".to_string(),
                output_item: Some(item_stack),
                input_items: vec![item_stack],
                required_equipment_id: 0,
                required_equipment_tier: 0,
                allow_use_hands: true,
                recipe_performance_id: 1,
            };

            if let Err(err) = ctx.db.item_conversion_recipe_desc().try_insert(item_list_conversion) {
                return Err(format!(
                    "Couldn't insert ItemConversionRecipeDesc for ItemDesc record with id {item_id}. Error message: {err}"
                ));
            }
        }
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn import_elevator_desc(ctx: &ReducerContext, records: Vec<ElevatorDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_elevator_desc_internal(ctx, records)?;
    Ok(())
}
fn import_elevator_desc_internal(ctx: &ReducerContext, records: Vec<ElevatorDesc>) -> Result<(), String> {
    for id in ctx.db.elevator_desc().iter().map(|item| item.building_id) {
        ctx.db.elevator_desc().building_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ElevatorDesc", len);
    for record in records {
        let id = record.building_id;
        if let Err(err) = ctx.db.elevator_desc().try_insert(record) {
            return Err(format!("Couldn't insert ElevatorDesc record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ElevatorDesc", len);
    Ok(())
}

fn generate_building_function_mappings(ctx: &ReducerContext) -> Result<(), String> {
    for id in ctx.db.building_function_type_mapping_desc().iter().map(|item| item.type_id) {
        ctx.db.building_function_type_mapping_desc().type_id().delete(&id);
    }

    for building_desc in ctx.db.building_desc().iter() {
        for function in building_desc.functions {
            let function_type = function.function_type;
            let mut mapping = match ctx.db.building_function_type_mapping_desc().type_id().find(&function_type) {
                Some(f) => f,
                None => ctx
                    .db
                    .building_function_type_mapping_desc()
                    .try_insert(BuildingFunctionTypeMappingDesc {
                        type_id: function_type,
                        desc_ids: Vec::new(),
                    })?,
            };
            mapping.desc_ids.push(building_desc.id);

            ctx.db.building_function_type_mapping_desc().type_id().update(mapping);
        }
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn import_player_action_desc(ctx: &ReducerContext, records: Vec<PlayerActionDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_player_action_desc_internal(ctx, records)?;
    Ok(())
}
fn import_player_action_desc_internal(ctx: &ReducerContext, records: Vec<PlayerActionDesc>) -> Result<(), String> {
    for id in ctx.db.player_action_desc().iter().map(|item| item.action_type_id) {
        ctx.db.player_action_desc().action_type_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PlayerActionDesc", len);
    for record in records {
        let id = record.action_type_id;
        if let Err(err) = ctx.db.player_action_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PlayerActionDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type PlayerActionDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_distant_visible_entity_desc(ctx: &ReducerContext, records: Vec<DistantVisibleEntityDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_distant_visible_entity_desc_internal(ctx, records)?;
    Ok(())
}
fn import_distant_visible_entity_desc_internal(ctx: &ReducerContext, records: Vec<DistantVisibleEntityDesc>) -> Result<(), String> {
    for id in ctx.db.distant_visible_entity_desc().iter().map(|item| item.id) {
        ctx.db.distant_visible_entity_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type DistantVisibleEntityDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.distant_visible_entity_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert DistantVisibleEntityDesc record with id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type DistantVisibleEntityDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_player_housing_desc(ctx: &ReducerContext, records: Vec<PlayerHousingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_player_housing_desc_internal(ctx, records)?;
    Ok(())
}
fn import_player_housing_desc_internal(ctx: &ReducerContext, records: Vec<PlayerHousingDesc>) -> Result<(), String> {
    for id in ctx.db.player_housing_desc().iter().map(|item| item.secondary_knowledge_id) {
        ctx.db.player_housing_desc().secondary_knowledge_id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PlayerHousingDesc", len);
    for record in records {
        let id = record.secondary_knowledge_id;
        if let Err(err) = ctx.db.player_housing_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PlayerHousingDesc record with secondary_knowledge_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type PlayerHousingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_hexite_exchange_entry_desc(ctx: &ReducerContext, records: Vec<HexiteExchangeEntryDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_hexite_exchange_entry_desc_internal(ctx, records)?;
    Ok(())
}
fn import_hexite_exchange_entry_desc_internal(ctx: &ReducerContext, records: Vec<HexiteExchangeEntryDesc>) -> Result<(), String> {
    for id in ctx.db.hexite_exchange_entry_desc().iter().map(|item| item.id) {
        ctx.db.hexite_exchange_entry_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type HexiteExchangeEntryDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.hexite_exchange_entry_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert HexiteExchangeEntryDesc record with secondary_knowledge_id {id}. Error message: {err}"
            ));
        }
    }
    log::info!("Inserted {} records of type HexiteExchangeEntryDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_reserved_name_desc(ctx: &ReducerContext, records: Vec<ReservedNameDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_reserved_name_desc_internal(ctx, records)?;
    Ok(())
}
fn import_reserved_name_desc_internal(ctx: &ReducerContext, records: Vec<ReservedNameDesc>) -> Result<(), String> {
    for id in ctx.db.reserved_name_desc().iter().map(|item| item.name) {
        ctx.db.reserved_name_desc().name().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ReservedName", len);
    for record in records {
        let id = record.name.clone();
        if let Err(err) = ctx.db.reserved_name_desc().try_insert(record) {
            return Err(format!("Couldn't insert ReservedName record with id {id}. Error message: {err}"));
        }
    }
    log::info!("Inserted {} records of type ReservedNamesDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_contribution_loot_desc(ctx: &ReducerContext, records: Vec<ContributionLootDescV2>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_contribution_loot_desc_internal(ctx, records)?;
    Ok(())
}
fn import_contribution_loot_desc_internal(ctx: &ReducerContext, records: Vec<ContributionLootDescV2>) -> Result<(), String> {
    for id in ctx
        .db
        .contribution_loot_desc_v2()
        .iter()
        .map(|item: ContributionLootDescV2| item.id)
    {
        ctx.db.contribution_loot_desc_v2().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ContributionLootDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.contribution_loot_desc_v2().try_insert(record) {
            return Err(format!(
                "Couldn't insert ContributionLootDescV2 record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type ContributionLootDescV2", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_wind_params_desc(ctx: &ReducerContext, records: Vec<WindParamsDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_wind_params_desc_internal(ctx, records)?;
    Ok(())
}
fn import_wind_params_desc_internal(ctx: &ReducerContext, records: Vec<WindParamsDesc>) -> Result<(), String> {
    for id in ctx.db.wind_params_desc().iter().map(|item: WindParamsDesc| item.id) {
        ctx.db.wind_params_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type WindParamsDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.wind_params_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert WindParamsDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type WindParamsDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_enemy_scaling_desc(ctx: &ReducerContext, records: Vec<EnemyScalingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_enemy_scaling_desc_internal(ctx, records)?;
    Ok(())
}
fn import_enemy_scaling_desc_internal(ctx: &ReducerContext, records: Vec<EnemyScalingDesc>) -> Result<(), String> {
    for id in ctx.db.enemy_scaling_desc().iter().map(|item: EnemyScalingDesc| item.id) {
        ctx.db.enemy_scaling_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type EnemyScalingDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.enemy_scaling_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert EnemyScalingDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type EnemyScalingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_premium_item_desc(ctx: &ReducerContext, records: Vec<PremiumItemDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_premium_item_desc_internal(ctx, records)?;
    Ok(())
}
fn import_premium_item_desc_internal(ctx: &ReducerContext, records: Vec<PremiumItemDesc>) -> Result<(), String> {
    for id in ctx.db.premium_item_desc().iter().map(|item: PremiumItemDesc| item.id) {
        ctx.db.premium_item_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PremiumItemDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.premium_item_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PremiumItemDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type PremiumItemDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_premium_service_desc(ctx: &ReducerContext, records: Vec<PremiumServiceDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_premium_service_desc_internal(ctx, records)?;
    Ok(())
}
fn import_premium_service_desc_internal(ctx: &ReducerContext, records: Vec<PremiumServiceDesc>) -> Result<(), String> {
    for id in ctx.db.premium_service_desc().iter().map(|item: PremiumServiceDesc| item.id) {
        ctx.db.premium_service_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type PremiumServiceDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.premium_service_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert PremiumServiceDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type PremiumServiceDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_ability_unlock_desc(ctx: &ReducerContext, records: Vec<AbilityUnlockDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_ability_unlock_desc_internal(ctx, records)?;
    Ok(())
}
fn import_ability_unlock_desc_internal(ctx: &ReducerContext, records: Vec<AbilityUnlockDesc>) -> Result<(), String> {
    for id in ctx.db.ability_unlock_desc().iter().map(|item: AbilityUnlockDesc| item.id) {
        ctx.db.ability_unlock_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type AbilityUnlockDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.ability_unlock_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert AbilityUnlockDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type AbilityUnlockDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_ability_custom_desc(ctx: &ReducerContext, records: Vec<AbilityCustomDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_ability_custom_desc_internal(ctx, records)?;
    Ok(())
}
fn import_ability_custom_desc_internal(ctx: &ReducerContext, records: Vec<AbilityCustomDesc>) -> Result<(), String> {
    for id in ctx.db.ability_custom_desc().iter().map(|item: AbilityCustomDesc| item.id) {
        ctx.db.ability_custom_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type AbilityCustomDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.ability_custom_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert AbilityCustomDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type AbilityCustomDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_prospecting_desc(ctx: &ReducerContext, records: Vec<ProspectingDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_prospecting_desc_internal(ctx, records)?;
    Ok(())
}
fn import_prospecting_desc_internal(ctx: &ReducerContext, records: Vec<ProspectingDesc>) -> Result<(), String> {
    for id in ctx.db.prospecting_desc().iter().map(|item: ProspectingDesc| item.id) {
        ctx.db.prospecting_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type ProspectingDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.prospecting_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert ProspectingDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type ProspectingDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_quest_chain_desc(ctx: &ReducerContext, records: Vec<QuestChainDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_quest_chain_desc_internal(ctx, records)?;
    Ok(())
}
fn import_quest_chain_desc_internal(ctx: &ReducerContext, records: Vec<QuestChainDesc>) -> Result<(), String> {
    for id in ctx.db.quest_chain_desc().iter().map(|item: QuestChainDesc| item.id) {
        ctx.db.quest_chain_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type QuestChainDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.quest_chain_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert QuestChainDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type QuestChainDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_stage_rewards_desc(ctx: &ReducerContext, records: Vec<StageRewardsDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_stage_rewards_desc_internal(ctx, records)?;
    Ok(())
}
fn import_stage_rewards_desc_internal(ctx: &ReducerContext, records: Vec<StageRewardsDesc>) -> Result<(), String> {
    for id in ctx.db.stage_rewards_desc().iter().map(|item: StageRewardsDesc| item.id) {
        ctx.db.stage_rewards_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type StageRewardsDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.stage_rewards_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert StageRewardsDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type StageRewardsDesc", len);
    Ok(())
}

#[spacetimedb::reducer]
pub fn import_quest_stage_desc(ctx: &ReducerContext, records: Vec<QuestStageDesc>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    import_quest_stage_desc_internal(ctx, records)?;
    Ok(())
}
fn import_quest_stage_desc_internal(ctx: &ReducerContext, records: Vec<QuestStageDesc>) -> Result<(), String> {
    for id in ctx.db.quest_stage_desc().iter().map(|item: QuestStageDesc| item.id) {
        ctx.db.quest_stage_desc().id().delete(&id);
    }
    let len: usize = records.len();
    log::info!("Will insert {} records of type QuestStageDesc", len);
    for record in records {
        let id = record.id;
        if let Err(err) = ctx.db.quest_stage_desc().try_insert(record) {
            return Err(format!(
                "Couldn't insert QuestStageDesc record with id {:?}. Error message: {}",
                id, err
            ));
        }
    }
    log::info!("Inserted {} records of type QuestStageDesc", len);
    Ok(())
}

fn collect_table<T: spacetimedb::Table>(table: &T) -> Vec<T::Row> {
    return table.iter().collect();
}

#[spacetimedb::reducer]
pub fn commit_staged_static_data(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    log::info!("Received instruction to commit staged data. Checking for empty vecs first...");

    validate_staged_data(ctx)?;

    let old_achievements = collect_table(ctx.db.achievement_desc());

    import_parameters_desc_internal(ctx, collect_table(ctx.db.staged_parameters_desc()))?;
    import_private_parameters_desc_internal(ctx, collect_table(ctx.db.staged_private_parameters_desc()))?;
    import_secondary_knowledge_desc_internal(ctx, collect_table(ctx.db.staged_secondary_knowledge_desc()))?;
    import_weapon_type_desc_internal(ctx, collect_table(ctx.db.staged_weapon_type_desc()))?;
    import_skill_desc_internal(ctx, collect_table(ctx.db.staged_skill_desc()))?;
    import_targeting_matrix_desc_internal(ctx, collect_table(ctx.db.staged_targeting_matrix_desc()))?;
    import_npc_desc_internal(ctx, collect_table(ctx.db.staged_npc_desc()))?;
    import_loot_rarity_desc_internal(ctx, collect_table(ctx.db.staged_loot_rarity_desc()))?;
    import_knowledge_scroll_type_desc_internal(ctx, collect_table(ctx.db.staged_knowledge_scroll_type_desc()))?;
    import_item_desc_internal(ctx, collect_table(ctx.db.staged_item_desc()))?;
    import_enemy_ai_params_desc_internal(ctx, collect_table(ctx.db.staged_enemy_ai_params_desc()))?;
    import_empire_rank_desc_internal(ctx, collect_table(ctx.db.staged_empire_rank_desc()))?;
    import_emote_desc_internal(ctx, collect_table(ctx.db.staged_emote_desc()))?;
    import_climb_requirement_desc_internal(ctx, collect_table(ctx.db.staged_climb_requirement_desc()))?;
    import_claim_tile_cost_internal(ctx, collect_table(ctx.db.staged_claim_tile_cost()))?;
    import_claim_tech_desc_internal(ctx, collect_table(ctx.db.staged_claim_tech_desc_v2()))?;
    import_character_stat_desc_internal(ctx, collect_table(ctx.db.staged_character_stat_desc()))?;
    import_cargo_desc_internal(ctx, collect_table(ctx.db.staged_cargo_desc()))?;
    import_building_type_desc_internal(ctx, collect_table(ctx.db.staged_building_type_desc()))?;
    import_building_desc_internal(ctx, collect_table(ctx.db.staged_building_desc()))?;
    import_biome_desc_internal(ctx, collect_table(ctx.db.staged_biome_desc()))?;
    import_interior_shape_desc_internal(ctx, collect_table(ctx.db.staged_interior_shape_desc()))?;
    import_buff_type_desc_internal(ctx, collect_table(ctx.db.staged_buff_type_desc()))?;
    import_buff_desc_internal(ctx, collect_table(ctx.db.staged_buff_desc()))?;
    import_alert_desc_internal(ctx, collect_table(ctx.db.staged_alert_desc()))?;
    import_tool_type_desc_internal(ctx, collect_table(ctx.db.staged_tool_type_desc()))?;
    import_item_list_desc_internal(ctx, collect_table(ctx.db.staged_item_list_desc()))?;
    import_food_desc_internal(ctx, collect_table(ctx.db.staged_food_desc()))?;
    import_achievement_desc_internal(ctx, collect_table(ctx.db.staged_achievement_desc()))?;
    import_knowledge_stat_modifier_desc_internal(ctx, collect_table(ctx.db.staged_knowledge_stat_modifier_desc()))?;
    import_interior_instance_desc_internal(ctx, collect_table(ctx.db.staged_interior_instance_desc()))?;
    import_interior_environment_desc_internal(ctx, collect_table(ctx.db.staged_interior_environment_desc()))?;
    import_interior_spawn_desc_internal(ctx, collect_table(ctx.db.staged_interior_spawn_desc()))?;
    import_building_portal_desc_v2_internal(ctx, collect_table(ctx.db.staged_building_portal_desc_v2()))?;
    import_interior_portal_connections_desc_internal(ctx, collect_table(ctx.db.staged_interior_portal_connections_desc()))?;
    import_interior_network_desc_internal(ctx, collect_table(ctx.db.staged_interior_network_desc()))?;
    import_building_claim_desc_internal(ctx, collect_table(ctx.db.staged_building_claim_desc()))?;
    import_building_repairs_desc_internal(ctx, collect_table(ctx.db.staged_building_repairs_desc()))?;
    import_building_spawn_desc_internal(ctx, collect_table(ctx.db.staged_building_spawn_desc()))?;
    import_chest_rarity_desc_internal(ctx, collect_table(ctx.db.staged_chest_rarity_desc()))?;
    import_clothing_desc_internal(ctx, collect_table(ctx.db.staged_clothing_desc()))?;
    import_collectible_desc_internal(ctx, collect_table(ctx.db.staged_collectible_desc()))?;
    import_combat_action_desc_v3_internal(ctx, collect_table(ctx.db.staged_combat_action_desc_v3()))?;
    import_combat_action_multi_hit_desc_internal(ctx, collect_table(ctx.db.staged_combat_action_multi_hit_desc()))?;
    import_construction_recipe_desc_internal(ctx, collect_table(ctx.db.staged_construction_recipe_desc_v2()))?;
    import_crafting_recipe_desc_internal(ctx, collect_table(ctx.db.staged_crafting_recipe_desc()))?;
    import_deconstruction_recipe_desc_internal(ctx, collect_table(ctx.db.staged_deconstruction_recipe_desc()))?;
    import_empire_supplies_desc_internal(ctx, collect_table(ctx.db.staged_empire_supplies_desc()))?;
    import_enemy_desc_internal(ctx, collect_table(ctx.db.staged_enemy_desc()))?;
    import_environment_debuff_desc_internal(ctx, collect_table(ctx.db.staged_environment_debuff_desc()))?;
    import_equipment_desc_internal(ctx, collect_table(ctx.db.staged_equipment_desc()))?;
    import_extraction_recipe_desc_internal(ctx, collect_table(ctx.db.staged_extraction_recipe_desc()))?;
    import_item_conversion_recipe_desc_internal(ctx, collect_table(ctx.db.staged_item_conversion_recipe_desc()))?;
    import_knowledge_scroll_desc_internal(ctx, collect_table(ctx.db.staged_knowledge_scroll_desc()))?;
    import_loot_chest_desc_internal(ctx, collect_table(ctx.db.staged_loot_chest_desc()))?;
    import_loot_table_desc_internal(ctx, collect_table(ctx.db.staged_loot_table_desc()))?;
    import_paving_tile_desc_internal(ctx, collect_table(ctx.db.staged_paving_tile_desc()))?;
    import_pillar_shaping_desc_internal(ctx, collect_table(ctx.db.staged_pillar_shaping_desc()))?;
    import_resource_desc_internal(ctx, collect_table(ctx.db.staged_resource_desc()))?;
    import_resource_clump_desc_internal(ctx, collect_table(ctx.db.staged_resource_clump_desc()))?;
    import_resource_growth_recipe_desc_internal(ctx, collect_table(ctx.db.staged_resource_growth_recipe_desc()))?;
    import_resource_placement_recipe_desc_internal(ctx, collect_table(ctx.db.staged_resource_placement_recipe_desc_v2()))?;
    import_teleport_item_desc_internal(ctx, collect_table(ctx.db.staged_teleport_item_desc()))?;
    import_tool_desc_internal(ctx, collect_table(ctx.db.staged_tool_desc()))?;
    import_traveler_task_desc_internal(ctx, collect_table(ctx.db.staged_traveler_task_desc()))?;
    import_traveler_trade_order_desc_internal(ctx, collect_table(ctx.db.staged_traveler_trade_order_desc()))?;
    import_deployable_desc_internal(ctx, collect_table(ctx.db.staged_deployable_desc()))?;
    import_weapon_desc_internal(ctx, collect_table(ctx.db.staged_weapon_desc()))?;
    import_onboarding_reward_desc_internal(ctx, collect_table(ctx.db.staged_onboarding_reward_desc()))?;
    import_terraform_recipe_desc_internal(ctx, collect_table(ctx.db.staged_terraform_recipe_desc()))?;
    import_empire_notification_desc_internal(ctx, collect_table(ctx.db.staged_empire_notification_desc()))?;
    import_empire_territory_desc_internal(ctx, collect_table(ctx.db.staged_empire_territory_desc()))?;
    import_empire_colors_desc_internal(ctx, collect_table(ctx.db.staged_empire_colors_desc()))?;
    import_empire_icon_desc_internal(ctx, collect_table(ctx.db.staged_empire_icon_desc()))?;
    import_wall_desc_internal(ctx, collect_table(ctx.db.staged_wall_desc()))?;
    import_gate_desc_internal(ctx, collect_table(ctx.db.staged_gate_desc()))?;
    import_pathfinding_desc_internal(ctx, collect_table(ctx.db.staged_pathfinding_desc()))?;
    import_elevator_desc_internal(ctx, collect_table(ctx.db.staged_elevator_desc()))?;
    import_player_action_desc_internal(ctx, collect_table(ctx.db.staged_player_action_desc()))?;
    import_distant_visible_entity_desc_internal(ctx, collect_table(ctx.db.staged_distant_visible_entity_desc()))?;
    import_player_housing_desc_internal(ctx, collect_table(ctx.db.staged_player_housing_desc()))?;
    import_hexite_exchange_entry_desc_internal(ctx, collect_table(ctx.db.staged_hexite_exchange_entry_desc()))?;
    import_contribution_loot_desc_internal(ctx, collect_table(ctx.db.staged_contribution_loot_desc()))?;
    import_wind_params_desc_internal(ctx, collect_table(ctx.db.staged_wind_params_desc()))?;
    import_enemy_scaling_desc_internal(ctx, collect_table(ctx.db.staged_enemy_scaling_desc()))?;
    import_premium_item_desc_internal(ctx, collect_table(ctx.db.staged_premium_item_desc()))?;
    import_premium_service_desc_internal(ctx, collect_table(ctx.db.staged_premium_service_desc()))?;
    import_ability_unlock_desc_internal(ctx, collect_table(ctx.db.staged_ability_unlock_desc()))?;
    import_ability_custom_desc_internal(ctx, collect_table(ctx.db.staged_ability_custom_desc()))?;
    import_prospecting_desc_internal(ctx, collect_table(ctx.db.staged_prospecting_desc()))?;
    import_quest_chain_desc_internal(ctx, collect_table(ctx.db.staged_quest_chain_desc()))?;
    import_stage_rewards_desc_internal(ctx, collect_table(ctx.db.staged_stage_rewards_desc()))?;
    import_quest_stage_desc_internal(ctx, collect_table(ctx.db.staged_quest_stage_desc()))?;

    import_static_data_post_processing(ctx)?;
    generate_building_function_mappings(ctx)?;
    if ctx.db.resources_log().iter().next().is_some() {
        //Don't update (start) agents before world is uploaded
        agents::update_scheduled_timers_from_static_data(ctx)?;
    }

    admin_update_light_source_states(ctx)?;
    migrate_achievements(ctx, &old_achievements)?;

    clear_staged_static_data(ctx)?;

    log::info!("Successfully committed staged static data.");
    Ok(())
}
