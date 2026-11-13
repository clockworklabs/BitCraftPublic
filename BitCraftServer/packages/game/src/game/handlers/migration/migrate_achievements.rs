use std::collections::HashMap;

use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::*,
        region::{migration_achievements_params, MigrationAchievementsParams},
        static_data::{achievement_desc, AchievementDesc},
    },
    unwrap_or_continue,
};

#[spacetimedb::reducer]
pub fn migration_set_achievement_params(ctx: &ReducerContext, allow_destructive: bool, grant_if_already_owned: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    ctx.db.migration_achievements_params().id().delete(0);
    ctx.db.migration_achievements_params().insert(MigrationAchievementsParams {
        id: 0,
        allow_destructive,
        grant_if_already_owned,
    });

    Ok(())
}

//This is called during static data upload
pub fn migrate_achievements(ctx: &ReducerContext, old_achievements: &[AchievementDesc]) -> Result<(), String> {
    let par = ctx
        .db
        .migration_achievements_params()
        .iter()
        .next()
        .unwrap_or(MigrationAchievementsParams {
            id: 0,
            allow_destructive: false,
            grant_if_already_owned: false,
        });
    ctx.db.migration_achievements_params().id().delete(par.id);

    let old_achievements: HashMap<i32, &AchievementDesc> = old_achievements.iter().map(|a| (a.id, a)).collect();
    let new_achievements: HashMap<i32, AchievementDesc> = ctx.db.achievement_desc().iter().map(|a| (a.id, a)).collect();
    let achievement_states: Vec<(u64, Vec<i32>)> = ctx
        .db
        .knowledge_achievement_state()
        .iter()
        .map(|s| {
            (
                s.entity_id,
                s.entries
                    .iter()
                    .filter(|e| e.state == KnowledgeState::Acquired)
                    .map(|e| e.id)
                    .collect(),
            )
        })
        .collect();

    if old_achievements.is_empty() {
        return Ok(()); //Initial static data upload
    }

    spacetimedb::log::info!(
        "Migrating achievements (allow destructive: {}, grant if already owned: {})",
        par.allow_destructive,
        par.grant_if_already_owned
    );

    for (achievement_id, new_achievement) in new_achievements {
        if let Some(old_achievement) = old_achievements.get(&achievement_id) {
            //Check newly added rewards
            for new_reward in &new_achievement.collectible_rewards {
                if !old_achievement.collectible_rewards.contains(new_reward) {
                    //Add new collectibles
                    spacetimedb::log::info!("Achievement {achievement_id} has new reward {new_reward}");
                    for (entity_id, achievement_state) in &achievement_states {
                        if achievement_state.contains(&achievement_id) {
                            if !par.grant_if_already_owned
                                && unwrap_or_continue!(
                                    ctx.db.vault_state().entity_id().find(entity_id),
                                    "Player {entity_id} has no vault state"
                                )
                                .collectibles
                                .iter()
                                .any(|c| c.id == *new_reward)
                            {
                                continue; //Player already has this reward
                            }

                            spacetimedb::log::info!("  Granting new reward {new_reward} to player {entity_id}");
                            VaultState::add_collectibles(ctx, *entity_id, vec![*new_reward]);
                        }
                    }
                }
            }

            //TODO Removing rewards not fully implemented
            //if par.allow_destructive {
            //    //Check removed rewards
            //    for old_reward in &old_achievement.collectible_rewards {
            //        if !new_achievement.collectible_rewards.contains(old_reward) {
            //            if let Some(collectible_desc) = ctx.db.collectible_desc().id().find(old_reward) {
            //                if collectible_desc.collectible_type == CollectibleType::Deployable {
            //                    spacetimedb::log::error!("Can't remove deployable {old_reward} (achievement {achievement_id}) from existing players: not implemented");
            //                    break;
            //                }
            //            }
            //
            //            //Remove collectibles
            //            spacetimedb::log::info!("Achievement {achievement_id} removed reward {old_reward}");
            //            for (entity_id, achievement_state) in &achievement_states {
            //                if achievement_state.contains(&achievement_id) {
            //                    let mut vault = unwrap_or_continue!(
            //                        ctx.db.vault_state().entity_id().find(entity_id),
            //                        "Player {entity_id} missing VaultState"
            //                    );
            //                    if let Some(index) = vault.collectibles.iter().position(|c| c.id == *old_reward) {
            //                        if vault.collectibles[index].count > 1 {
            //                            vault.collectibles[index].count -= 1;
            //                        } else {
            //                            //TODO what if it's equipped?
            //                            vault.collectibles.remove(index);
            //                        }
            //                        ctx.db.vault_state().entity_id().update(vault);
            //                    }
            //                }
            //            }
            //        }
            //    }
            //}
        }
    }

    spacetimedb::log::info!("Achievements migrated");

    Ok(())
}
