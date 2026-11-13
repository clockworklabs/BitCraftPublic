use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{game::autogen::_delete_entity, messages::authentication::ServerIdentity};

#[spacetimedb::table(accessor = player_vote_conclude_timer, scheduled(player_vote_conclude, at = scheduled_at))]
pub struct PlayerVoteConcludeTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub vote_entity_id: u64,
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_vote_conclude(ctx: &ReducerContext, timer: PlayerVoteConcludeTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    reduce(ctx, timer.vote_entity_id)
}

pub fn reduce(ctx: &ReducerContext, vote_entity_id: u64) -> Result<(), String> {
    _delete_entity::delete_entity(ctx, vote_entity_id);
    Ok(())
}
