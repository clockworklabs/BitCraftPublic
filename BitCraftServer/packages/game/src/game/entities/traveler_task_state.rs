use std::collections::{HashMap, HashSet};

use spacetimedb::{rand::Rng, ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{
        components::{experience_state, knowledge_secondary_state, traveler_task_state, TravelerTaskCreditState, TravelerTaskState},
        static_data::{
            npc_desc, parameters_desc, traveler_task_desc, traveler_task_knowledge_requirement_desc, TravelerTaskDesc,
            TravelerTaskKnowledgeRequirementDesc,
        },
    },
};

impl TravelerTaskState {
    pub fn create_and_commit(ctx: &ReducerContext, player_entity_id: u64, traveler_id: i32, task_id: i32) {
        let entity_id = game_state::create_entity(ctx);
        let traveler_task = TravelerTaskState {
            entity_id,
            player_entity_id,
            traveler_id,
            task_id,
            completed: false,
        };
        ctx.db.traveler_task_state().insert(traveler_task);
    }

    pub fn generate_initial_tasks(ctx: &ReducerContext, player_entity_id: u64) {
        let requests = Self::generate_npc_requests_hashmap(ctx);
        let max_tasks_per_traveler = ctx
            .db
            .parameters_desc()
            .version()
            .find(0)
            .map(|params| params.traveler_tasks_per_npc.max(0) as usize)
            .unwrap_or_default();

        if max_tasks_per_traveler == 0 {
            return;
        }

        for npc in ctx.db.npc_desc().iter() {
            if npc.task_skill_check.is_empty() || npc.task_skill_check.iter().all(|skill| *skill <= 0) {
                continue;
            }

            let traveler_id = npc.npc_type;
            let mut excluded_task_ids = Self::active_task_ids_for_traveler(ctx, player_entity_id, traveler_id, None);

            while excluded_task_ids.len() < max_tasks_per_traveler {
                let Some(task_id) = Self::roll_task_id(ctx, player_entity_id, traveler_id, &requests, &excluded_task_ids) else {
                    break;
                };

                Self::create_and_commit(ctx, player_entity_id, traveler_id, task_id);
                excluded_task_ids.insert(task_id);
            }
        }
    }

    pub fn generate_npc_requests_hashmap(ctx: &ReducerContext) -> HashMap<i32, Vec<TravelerTaskDesc>> {
        let mut npc_requests = HashMap::new();
        for npc in ctx.db.npc_desc().iter() {
            let npc_tasks = ctx
                .db
                .traveler_task_desc()
                .iter()
                .filter(|t| npc.task_skill_check.contains(&t.level_requirement.skill_id))
                .collect();
            npc_requests.insert(npc.npc_type, npc_tasks);
        }
        npc_requests
    }

    pub fn replace_completed_task(
        ctx: &ReducerContext,
        player_entity_id: u64,
        task_entity_id: u64,
        traveler_id: i32,
        completed_task_id: i32,
        cost: u32,
        requests: &HashMap<i32, Vec<TravelerTaskDesc>>,
    ) -> bool {
        let mut excluded_task_ids = Self::active_task_ids_for_traveler(ctx, player_entity_id, traveler_id, Some(task_entity_id));
        excluded_task_ids.insert(completed_task_id);

        let Some(next_task_id) = Self::roll_task_id(ctx, player_entity_id, traveler_id, requests, &excluded_task_ids) else {
            return false;
        };

        if cost > 0 && !TravelerTaskCreditState::try_consume_credits(ctx, player_entity_id, traveler_id, cost) {
            return false;
        }

        if let Some(mut task_state) = ctx.db.traveler_task_state().entity_id().find(task_entity_id) {
            task_state.task_id = next_task_id;
            task_state.completed = false;
            ctx.db.traveler_task_state().entity_id().update(task_state);
            return true;
        }

        false
    }

    pub fn retry_completed_task_replacements(
        ctx: &ReducerContext,
        player_entity_id: u64,
        refreshed_traveler_ids: &HashSet<i32>,
    ) {
        let completed_tasks: Vec<TravelerTaskState> = ctx
            .db
            .traveler_task_state()
            .player_entity_id()
            .filter(player_entity_id)
            .filter(|task| task.completed)
            .filter(|task| refreshed_traveler_ids.contains(&task.traveler_id))
            .collect();

        if completed_tasks.len() == 0 {
            return;
        }

        let requests = Self::generate_npc_requests_hashmap(ctx);
        for task in completed_tasks {
            let _ = Self::replace_completed_task(ctx, player_entity_id, task.entity_id, task.traveler_id, task.task_id, 0, &requests);
        }
    }

    pub fn reroll_task(ctx: &ReducerContext, task_entity_id: u64) -> Result<(), String> {
        let mut task_state = ctx
            .db
            .traveler_task_state()
            .entity_id()
            .find(task_entity_id)
            .ok_or_else(|| "This task no longer exists".to_string())?;

        let player_entity_id = task_state.player_entity_id;
        let traveler_id = task_state.traveler_id;
        let previous_task_id = task_state.task_id;

        let requests = Self::generate_npc_requests_hashmap(ctx);
        let mut excluded_task_ids = Self::active_task_ids_for_traveler(ctx, player_entity_id, traveler_id, Some(task_entity_id));
        excluded_task_ids.insert(previous_task_id);

        let next_task_id = Self::roll_task_id(ctx, player_entity_id, traveler_id, &requests, &excluded_task_ids)
            .ok_or_else(|| "No replacement traveler task is available".to_string())?;

        let reroll_cost = TravelerTaskCreditState::reroll_credit_cost(ctx);
        if !TravelerTaskCreditState::try_consume_credits(ctx, player_entity_id, traveler_id, reroll_cost) {
            return Err("You do not have enough traveler task credits to reroll this task".into());
        }

        task_state.task_id = next_task_id;
        task_state.completed = false;
        ctx.db.traveler_task_state().entity_id().update(task_state);
        Ok(())
    }

    fn active_task_ids_for_traveler(
        ctx: &ReducerContext,
        player_entity_id: u64,
        traveler_id: i32,
        excluded_entity_id: Option<u64>,
    ) -> HashSet<i32> {
        ctx.db
            .traveler_task_state()
            .per_player_and_traveler_id()
            .filter((player_entity_id, traveler_id))
            .filter(|task| excluded_entity_id != Some(task.entity_id))
            .map(|task| task.task_id)
            .collect()
    }

    fn roll_task_id(
        ctx: &ReducerContext,
        player_entity_id: u64,
        traveler_id: i32,
        requests: &HashMap<i32, Vec<TravelerTaskDesc>>,
        excluded_task_ids: &HashSet<i32>,
    ) -> Option<i32> {
        let available_tasks = requests.get(&traveler_id)?;
        if available_tasks.is_empty() {
            return None;
        }

        let player_knowledges = ctx.db.knowledge_secondary_state().entity_id().find(player_entity_id)?;
        let knowledge_requirements: HashMap<i32, TravelerTaskKnowledgeRequirementDesc> = ctx
            .db
            .traveler_task_knowledge_requirement_desc()
            .iter()
            .map(|requirements| (requirements.traveler_task_id, requirements))
            .collect();
        let experience = ctx.db.experience_state().entity_id().find(player_entity_id)?;

        let mut skill_appropriate_task_pool: Vec<i32> = available_tasks
            .iter()
            .filter(|task| !excluded_task_ids.contains(&task.id))
            .filter(|task| {
                let level = experience.get_level(task.level_requirement.skill_id);
                level >= task.level_requirement.min_level && level <= task.level_requirement.max_level
            })
            .filter(|task| match knowledge_requirements.get(&task.id) {
                Some(requirements) => {
                    requirements.required_knowledges.is_empty()
                        || requirements
                            .required_knowledges
                            .iter()
                            .all(|knowledge_id| player_knowledges.is_acquired(*knowledge_id))
                }
                None => true,
            })
            .filter(|task| match knowledge_requirements.get(&task.id) {
                Some(requirements) => {
                    requirements.blocking_knowledges.is_empty()
                        || !requirements
                            .blocking_knowledges
                            .iter()
                            .any(|knowledge_id| player_knowledges.is_acquired(*knowledge_id))
                }
                None => true,
            })
            .map(|task| task.id)
            .collect();

        if skill_appropriate_task_pool.is_empty() {
            return None;
        }

        let rnd = ctx.rng().gen_range(0..skill_appropriate_task_pool.len());
        Some(skill_appropriate_task_pool.swap_remove(rnd))
    }
}
