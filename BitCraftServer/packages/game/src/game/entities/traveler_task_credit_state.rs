use std::collections::HashSet;

use spacetimedb::{ReducerContext, Table};

use crate::{
    agents::traveler_task_agent,
    game::game_state,
    messages::{
        components::{traveler_task_credit_state, TravelerTaskCreditState, TravelerTaskState},
        static_data::{npc_desc, parameters_desc},
    },
};

impl TravelerTaskCreditState {
    pub fn refresh_player_credits(ctx: &ReducerContext, player_entity_id: u64) -> bool {
        let last_tick = traveler_task_agent::last_tick(ctx);
        let mut refreshed_traveler_ids = HashSet::new();

        for npc in ctx.db.npc_desc().iter().filter(|x| x.task_skill_check.len() > 0) {
            match ctx
                .db
                .traveler_task_credit_state()
                .player_and_traveler_id()
                .filter((player_entity_id, npc.npc_type))
                .next()
            {
                Some(mut credit_state) => {
                    if Self::should_reset(game_state::unix(ctx.timestamp), credit_state.last_reset) {
                        credit_state.credits = Self::weekly_credit_count(ctx, npc.npc_type);
                        credit_state.last_reset = last_tick;
                        ctx.db.traveler_task_credit_state().entity_id().update(credit_state);
                        refreshed_traveler_ids.insert(npc.npc_type);
                    }
                }
                None => {
                    ctx.db.traveler_task_credit_state().insert(TravelerTaskCreditState {
                        entity_id: game_state::create_entity(ctx),
                        player_entity_id,
                        traveler_id: npc.npc_type,
                        credits: Self::weekly_credit_count(ctx, npc.npc_type),
                        last_reset: last_tick,
                    });
                    refreshed_traveler_ids.insert(npc.npc_type);
                }
            }
        }

        if !refreshed_traveler_ids.is_empty() {
            TravelerTaskState::retry_completed_task_replacements(
                ctx,
                player_entity_id,
                &refreshed_traveler_ids,
            );
        }

        !refreshed_traveler_ids.is_empty()
    }

    pub fn reroll_credit_cost(ctx: &ReducerContext) -> u32 {
        ctx.db
            .parameters_desc()
            .version()
            .find(0)
            .map(|params| params.traveler_task_reroll_credit_cost.max(0) as u32)
            .unwrap_or_default()
    }

    pub fn try_consume_credits(ctx: &ReducerContext, player_entity_id: u64, traveler_id: i32, cost: u32) -> bool {
        let Some(mut credit_state) = ctx
            .db
            .traveler_task_credit_state()
            .player_and_traveler_id()
            .filter((player_entity_id, traveler_id))
            .next()
        else {
            return false;
        };

        if credit_state.credits < cost {
            return false;
        }

        credit_state.credits -= cost;
        ctx.db.traveler_task_credit_state().entity_id().update(credit_state);
        true
    }

    fn weekly_credit_count(ctx: &ReducerContext, traveler_id: i32) -> u32 {
        ctx.db
            .parameters_desc()
            .version()
            .find(0)
            .and_then(|params| {
                params
                    .traveler_task_weekly_credits
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .find(|credit| credit.traveler_type as i32 == traveler_id)
                    .map(|credit| credit.weekly_task_credits.max(0) as u32)
            })
            .unwrap_or_default()
    }

    fn should_reset(now: i32, last_reset: i32) -> bool {
        now.saturating_sub(last_reset) >= traveler_task_agent::TRAVELER_TASK_RESET_INTERVAL_SECS
    }
}
