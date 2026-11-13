use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimApplyForRecruitmentRequest, components::*, game_util::LevelRequirement},
    unwrap_or_err,
};

use super::claim_add_member;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_apply_for_recruitment(ctx: &ReducerContext, request: PlayerClaimApplyForRecruitmentRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut recruitment_order = unwrap_or_err!(
        ctx.db.claim_recruitment_state().entity_id().find(request.recruitment_entity_id),
        "Recruitment offer no longer exists."
    )
    .clone();

    if recruitment_order.remaining_stock <= 0 {
        return Err("This recruitment order is expired.".into());
    }

    let req = LevelRequirement {
        skill_id: recruitment_order.required_skill_id,
        level: recruitment_order.required_skill_level,
    };

    if !PlayerState::meets_level_requirement(ctx, actor_id, &req) {
        return Err("You are not eligible".into());
    }

    // todo, recruitment order "require approval"

    // join claim
    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(recruitment_order.claim_entity_id),
        "No such claim."
    );
    claim_add_member::reduce(ctx, claim, recruitment_order.claim_entity_id, actor_id)?;

    // Successfully joined the claim, update order
    recruitment_order.remaining_stock -= 1;
    ctx.db.claim_recruitment_state().entity_id().update(recruitment_order);

    Ok(())
}
