use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::messages::action_request::PlayerAchievementClaimRequest;
use crate::messages::static_data::AchievementDesc;
use crate::PlayerTimestampState;

#[spacetimedb::reducer]
#[feature_gate]
pub fn achievement_claim(ctx: &ReducerContext, request: PlayerAchievementClaimRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    if !AchievementDesc::acquire(ctx, actor_id, request.achievement_id) {
        return Err("Requirements not met.".into());
    }

    Ok(())
}
