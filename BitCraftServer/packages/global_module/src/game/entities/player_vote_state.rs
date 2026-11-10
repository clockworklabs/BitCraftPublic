use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        game_state::create_entity,
        handlers::player::player_vote_conclude::{player_vote_conclude_timer, PlayerVoteConcludeTimer},
        reducer_helpers::timer_helpers::now_plus_secs_f32,
    },
    messages::{
        components::player_username_state,
        empire_shared::*,
        global::{player_vote_state, PlayerVoteAnswer, PlayerVoteState, PlayerVoteType},
    },
};

impl PlayerVoteState {
    pub fn new(
        ctx: &ReducerContext,
        vote_type: PlayerVoteType,
        initiator_entity_id: u64,
        participants_entity_id: Vec<u64>,
        default_initiator_vote: bool,
        pass_threshold: f32,
        argument1: u64,
        argument2: u64,
    ) -> Option<PlayerVoteState> {
        if let Some(player_name_state) = ctx.db.player_username_state().entity_id().find(&initiator_entity_id) {
            let initiator_name = player_name_state.username;
            let len = participants_entity_id.len();
            let entity_id = create_entity(ctx);
            let mut vote = PlayerVoteState {
                entity_id,
                vote_type,
                initiator_entity_id,
                participants_entity_id,
                answers: vec![PlayerVoteAnswer::None; len],
                initiator_name,
                pass_threshold,
                outcome: PlayerVoteAnswer::None,
                argument1,
                argument2,
                outcome_str: String::new(),
            };
            if default_initiator_vote {
                if let Some(index) = vote.participants_entity_id.iter().position(|p| *p == initiator_entity_id) {
                    vote.answers[index] = PlayerVoteAnswer::Yes;
                }
            }
            return Some(vote);
        }
        None
    }

    pub fn insert_with_end_timer(
        ctx: &ReducerContext,
        vote_type: PlayerVoteType,
        initiator_entity_id: u64,
        participants_entity_id: Vec<u64>,
        default_initiator_vote: bool,
        pass_threshold: f32,
        duration: f32,
        argument1: u64,
        argument2: u64,
    ) {
        if let Some(teleport_request) = PlayerVoteState::new(
            ctx,
            vote_type,
            initiator_entity_id,
            participants_entity_id,
            default_initiator_vote,
            pass_threshold,
            argument1,
            argument2,
        ) {
            let vote_entity_id = teleport_request.entity_id;
            if ctx.db.player_vote_state().try_insert(teleport_request).is_err() {
                log::error!("Error inserting new vote");
                return;
            }

            ctx.db
                .player_vote_conclude_timer()
                .try_insert(PlayerVoteConcludeTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(duration, ctx.timestamp),
                    vote_entity_id,
                })
                .ok()
                .unwrap();
        }
    }

    pub fn play_outcome(&mut self, ctx: &ReducerContext) -> Result<(), String> {
        match self.vote_type {
            PlayerVoteType::JoinEmpire => self.join_empire(ctx)?,
            PlayerVoteType::SubmitEmpire => self.join_empire(ctx)?,
            PlayerVoteType::SubmitWatchtower => self.submit_watchtower(ctx),
        }
        Ok(())
    }

    fn join_empire(&mut self, ctx: &ReducerContext) -> Result<(), String> {
        if self.outcome == PlayerVoteAnswer::Yes {
            let acceptor_entity_id = self
                .participants_entity_id
                .iter()
                .find(|p| **p != self.initiator_entity_id)
                .unwrap();

            if self.vote_type == PlayerVoteType::SubmitEmpire {
                if let Some(empire) = ctx.db.empire_state().capital_building_entity_id().find(self.argument1) {
                    if let Some(emperor) = ctx.db.empire_player_data_state().entity_id().find(acceptor_entity_id) {
                        log::info!("Merging empire {} into {}", empire.entity_id, emperor.empire_entity_id);
                        empire.merge_into(ctx, emperor.empire_entity_id)?;
                    }
                }
            } else {
                // DAB Note: Remove PlayerVoteType::JoinEmpire
            }
        } else {
            self.outcome_str = "Owner declined the invitation.".to_string();
            log::info!("Join empire error - {}", self.outcome_str);
        }
        Ok(())
    }

    fn submit_watchtower(&mut self, ctx: &ReducerContext) {
        if self.outcome == PlayerVoteAnswer::Yes {
            let acceptor_entity_id = self
                .participants_entity_id
                .iter()
                .find(|p| **p != self.initiator_entity_id)
                .unwrap();

            self.outcome_str = "Something went wrong".to_string();
            if let Some(acceptor_emperor) = ctx.db.empire_player_data_state().entity_id().find(acceptor_entity_id) {
                let new_empire_entity_id = acceptor_emperor.empire_entity_id;
                let watchtower_entity_id = self.argument1;
                if let Some(mut empire_node) = ctx.db.empire_node_state().entity_id().find(watchtower_entity_id) {
                    let previous_empire_entity_id = empire_node.empire_entity_id;
                    empire_node.convert(ctx, empire_node.energy, new_empire_entity_id);
                    EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
                    EmpireState::update_empire_upkeep(ctx, previous_empire_entity_id);
                    EmpireState::update_empire_upkeep(ctx, new_empire_entity_id);
                    self.outcome_str = "The offer has been accepted".to_string();
                }
            }
        } else {
            self.outcome_str = "The offer has been declined".to_string();
        }
    }
}
