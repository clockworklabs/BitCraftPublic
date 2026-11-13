use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimRemoveRecruitmentRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_remove_recruitment(ctx: &ReducerContext, request: PlayerClaimRemoveRecruitmentRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let recruitment_state = unwrap_or_err!(
        ctx.db.claim_recruitment_state().entity_id().find(request.recruitment_entity_id),
        "No such recruitment order."
    );

    let claim = unwrap_or_err!(
        ctx.db
            .claim_state()
            .entity_id()
            .find(recruitment_state.claim_entity_id),
        "Recruitment's claim do not exist anymore."
    );

    if !claim.has_officer_permissions(ctx, actor_id) {
        return Err("You don't have permission to edit recruitment orders.".into());
    }

    ctx.db.claim_recruitment_state().entity_id().delete(request.recruitment_entity_id);

    Ok(())
}
