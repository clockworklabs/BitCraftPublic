use spacetimedb::{ReducerContext, Table};

use crate::{
    achievement_desc,
    game::discovery::Discovery,
    messages::{components::*, generic::RegionExplorationInfo},
    AchievementDesc,
};

impl AchievementDesc {
    fn known_chunks_pct(ctx: &ReducerContext, known_chunks: i32) -> f32 {
        let eligible_world_chunk_count = RegionExplorationInfo::world_achievement_chunk_count(ctx);
        if eligible_world_chunk_count == 0 {
            0.0
        } else {
            known_chunks as f32 / eligible_world_chunk_count as f32
        }
    }

    pub fn discover_eligible(ctx: &ReducerContext, entity_id: u64) {
        // Discover any achievement that isn't already acquired and whose achievement requirements are met
        let known_achievements = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap();

        let mut discovered_achievements = Vec::new();

        // Find all non-discovered achievements that could be tracked based on requirements
        for achievement in ctx
            .db
            .achievement_desc()
            .iter()
            .filter(|a| !known_achievements.entries.iter().any(|ka| ka.id == a.id))
        {
            let mut all_met = true;
            for sought_id in achievement.requisites {
                all_met &= known_achievements
                    .entries
                    .iter()
                    .any(|a| a.id == sought_id && a.state == KnowledgeState::Acquired);
            }
            if all_met {
                discovered_achievements.push(achievement.id);
            }
        }

        // no new achievement to track, early exit.
        if discovered_achievements.is_empty() {
            return;
        }

        let mut discovery = Discovery::new(entity_id);
        for achievement_id in discovered_achievements {
            discovery.discover_achievement(ctx, achievement_id);
        }
        // we want to re-evaluate achievements since new ones came in the pool
        discovery.acquired_achievement = true;

        // this will evaluate these newly discovered achievements.
        discovery.commit(ctx);
    }

    #[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
    fn evaluate_achievement(
        achievement: &AchievementDesc,
        experience: &ExperienceState,
        known_resources: &[KnowledgeEntry],
        known_crafts: &[KnowledgeEntry],
        known_cargos: &[KnowledgeEntry],
        known_items: &[KnowledgeEntry],
        known_achievements: &[KnowledgeEntry],
        known_chunks: i32,
        known_chunks_pct: f32,
    ) -> bool {
        for req_achievement in &achievement.requisites {
            let known = known_achievements.iter().find(|k| k.id == *req_achievement);
            if known.is_none() || known.unwrap().state != KnowledgeState::Acquired {
                return false;
            }
        }

        if achievement.skill_id != 0
            && experience.get_level(achievement.skill_id) < achievement.skill_level {
                return false;
            }

        for discovery in &achievement.resource_disc {
            let known = known_resources.iter().find(|k| k.id == *discovery);
            if known.is_none() || known.unwrap().state != KnowledgeState::Acquired {
                return false;
            }
        }

        for discovery in &achievement.crafting_disc {
            let known = known_crafts.iter().find(|k| k.id == *discovery);
            if known.is_none() || known.unwrap().state != KnowledgeState::Acquired {
                return false;
            }
        }

        for discovery in &achievement.cargo_disc {
            let known = known_cargos.iter().find(|k| k.id == *discovery);
            if known.is_none() || known.unwrap().state != KnowledgeState::Acquired {
                return false;
            }
        }

        for discovery in &achievement.item_disc {
            let known = known_items.iter().find(|k| k.id == *discovery);
            if known.is_none() || known.unwrap().state != KnowledgeState::Acquired {
                return false;
            }
        }

        if achievement.chunks_discovered > known_chunks {
            return false;
        }

        if achievement.pct_chunks_discovered > known_chunks_pct {
            return false;
        }

        true
    }

    pub fn evaluate_all(_entity_id: u64) {
        /*
        let known_achievements =
            ctx.db.knowledge_achievement_state().entity_id().find(&entity_id).unwrap();

        // caching knowledges for faster evaluation
        let known_resources = ctx.db.knowledge_resource_state().entity_id().find(&entity_id).unwrap().entries;
        let known_crafts = ctx.db.knowledge_craft_state().entity_id().find(&entity_id).unwrap().entries;
        let known_cargos = ctx.db.knowledge_cargo_state().entity_id().find(&entity_id).unwrap().entries;
        let known_items = ctx.db.knowledge_item_state().entity_id().find(&entity_id).unwrap().entries;
        let experience_state = ctx.db.experience_state().entity_id().find(&entity_id).unwrap();

        let mut acquired_achievements = Vec::new();
        // Evaluate all "discovered" achievements.
        for achievement_knowledge in known_achievements
            .entries
            .iter()
            .filter(|e| e.state == knowledge_entry::State::Discovered)
        {
            let achievement = ctx.db.achievement().id().find(&achievement_knowledge.id).unwrap();
            if Self::evaluate_achievement(
                &achievement,
                &experience_state,
                &known_resources,
                &known_crafts,
                &known_cargos,
                &known_items,
            ) {
                acquired_achievements.push(achievement.id);

                VaultState::add_collectibles(entity_id, achievement.collectible_rewards);
            }
        }

        if acquired_achievements.len() > 0 {
            let mut discovery = Discovery::new(entity_id);
            for acq_ach in acquired_achievements {
                discovery.acquire_achievement(acq_ach);
            }
            discovery.commit();
            Self::discover_eligible(entity_id);
        }
        */
    }

    // This is basically evaluate_all but filtering with a specific achievement_id
    pub fn acquire(ctx: &ReducerContext, entity_id: u64, achievement_id: i32) -> bool {
        let mut known_achievements = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap();

        // caching knowledges for faster evaluation
        let known_resources = ctx.db.knowledge_resource_state().entity_id().find(entity_id).unwrap().entries;
        let known_crafts = ctx.db.knowledge_craft_state().entity_id().find(entity_id).unwrap().entries;
        let known_cargos = ctx.db.knowledge_cargo_state().entity_id().find(entity_id).unwrap().entries;
        let known_items = ctx.db.knowledge_item_state().entity_id().find(entity_id).unwrap().entries;
        let known_achievements_cache = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap().entries;

        let experience_state = ctx.db.experience_state().entity_id().find(entity_id).unwrap();
        let known_chunks = ctx
            .db
            .exploration_chunks_state_v2()
            .entity_id()
            .find(entity_id)
            .unwrap()
            .achievement_explored_chunks_count();

        let known_chunks_pct = Self::known_chunks_pct(ctx, known_chunks);

        let mut acquired_achievements = Vec::new();

        // Discover achievement if not already discovered
        if !known_achievements
            .entries
            .iter()
            .any(|a| a.id == achievement_id && a.state != KnowledgeState::Unknown)
        {
            let mut discovery = Discovery::new(entity_id);
            discovery.discover_achievement(ctx, achievement_id);

            // this will evaluate these newly discovered achievements.
            discovery.commit(ctx);
            known_achievements = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap();
        }

        // Evaluate all "discovered" achievements.
        for achievement_knowledge in known_achievements
            .entries
            .iter()
            .filter(|e| e.state == KnowledgeState::Discovered && e.id == achievement_id)
        {
            let achievement = ctx.db.achievement_desc().id().find(achievement_knowledge.id).unwrap();
            if Self::evaluate_achievement(
                &achievement,
                &experience_state,
                &known_resources,
                &known_crafts,
                &known_cargos,
                &known_items,
                &known_achievements_cache,
                known_chunks,
                known_chunks_pct,
            ) {
                acquired_achievements.push(achievement.id);
                VaultState::add_collectibles(ctx, entity_id, achievement.collectible_rewards);
            }
        }

        if !acquired_achievements.is_empty() {
            let mut discovery = Discovery::new(entity_id);
            for acq_ach in acquired_achievements {
                discovery.acquire_achievement(ctx, acq_ach);
            }
            discovery.commit(ctx);
            Self::discover_eligible(ctx, entity_id);
            true
        } else {
            false
        }
    }

    pub fn evaluate_achievements(ctx: &ReducerContext, player_entity_id: u64, required_achievements: Vec<i32>) -> bool {
        if required_achievements.is_empty() {
            return true;
        }

        // caching knowledges for faster evaluation
        let known_resources = ctx
            .db
            .knowledge_resource_state()
            .entity_id()
            .find(player_entity_id)
            .unwrap()
            .entries;
        let known_crafts = ctx.db.knowledge_craft_state().entity_id().find(player_entity_id).unwrap().entries;
        let known_cargos = ctx.db.knowledge_cargo_state().entity_id().find(player_entity_id).unwrap().entries;
        let known_items = ctx.db.knowledge_item_state().entity_id().find(player_entity_id).unwrap().entries;
        let experience_state = ctx.db.experience_state().entity_id().find(player_entity_id).unwrap();
        let known_achievements = ctx
            .db
            .knowledge_achievement_state()
            .entity_id()
            .find(player_entity_id)
            .unwrap()
            .entries;

        let known_chunks = ctx
            .db
            .exploration_chunks_state_v2()
            .entity_id()
            .find(player_entity_id)
            .unwrap()
            .achievement_explored_chunks_count();

        let known_chunks_pct = Self::known_chunks_pct(ctx, known_chunks);

        for required_achievement_id in required_achievements {
            let achievement = ctx.db.achievement_desc().id().find(required_achievement_id).unwrap();

            if !Self::evaluate_achievement(
                &achievement,
                &experience_state,
                &known_resources,
                &known_crafts,
                &known_cargos,
                &known_items,
                &known_achievements,
                known_chunks,
                known_chunks_pct,
            ) {
                return false;
            }
        }

        true
    }

    pub fn meets_requirements_for_achievement(ctx: &ReducerContext, entity_id: u64, achievement_id: i32) -> bool {
        // caching knowledges for faster evaluation
        let known_resources = ctx.db.knowledge_resource_state().entity_id().find(entity_id).unwrap().entries;
        let known_crafts = ctx.db.knowledge_craft_state().entity_id().find(entity_id).unwrap().entries;
        let known_cargos = ctx.db.knowledge_cargo_state().entity_id().find(entity_id).unwrap().entries;
        let known_items = ctx.db.knowledge_item_state().entity_id().find(entity_id).unwrap().entries;
        let known_achievements = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap().entries;

        let experience_state = ctx.db.experience_state().entity_id().find(entity_id).unwrap();
        let known_chunks = ctx
            .db
            .exploration_chunks_state_v2()
            .entity_id()
            .find(entity_id)
            .unwrap()
            .achievement_explored_chunks_count();

        let known_chunks_pct = Self::known_chunks_pct(ctx, known_chunks);

        let achievement = ctx.db.achievement_desc().id().find(achievement_id).unwrap();
        Self::evaluate_achievement(
            &achievement,
            &experience_state,
            &known_resources,
            &known_crafts,
            &known_cargos,
            &known_items,
            &known_achievements,
            known_chunks,
            known_chunks_pct,
        )
    }
}
