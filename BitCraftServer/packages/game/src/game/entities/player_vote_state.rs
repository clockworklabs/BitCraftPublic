use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        game_state::create_entity,
        handlers::player::player_vote_conclude::{player_vote_conclude_timer, PlayerVoteConcludeTimer},
        reducer_helpers::timer_helpers::now_plus_secs_f32,
    },
    messages::{
        components::{player_username_state, DuelState},
        region::*,
    },
};

impl PlayerVoteState {
    #[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
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
        if let Some(player_name_state) = ctx.db.player_username_state().entity_id().find(initiator_entity_id) {
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

    #[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
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

    pub fn play_outcome(&mut self, ctx: &ReducerContext) {
        match self.vote_type {
            PlayerVoteType::Duel => self.duel(ctx),
        }
    }

    fn duel(&mut self, ctx: &ReducerContext) {
        if self.outcome == PlayerVoteAnswer::Yes {
            let _ = DuelState::initiate(ctx, self.participants_entity_id[0], self.participants_entity_id[1]);
        } else {
            self.outcome_str = "Player declined the duel.".to_string();
            log::info!("Duel declined - {}", self.outcome_str);
        }
    }
}
