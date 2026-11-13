use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimAddRecruitmentRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_add_recruitment(ctx: &ReducerContext, request: PlayerClaimAddRecruitmentRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(request.claim_entity_id),
        "Recruitment's claim do not exist anymore."
    );

    if !claim.has_officer_permissions(ctx, actor_id) {
        return Err("You don't have permission to edit recruitment orders.".into());
    }

    let entity_id = game_state::create_entity(ctx);
    let recruitment_offer = ClaimRecruitmentState {
        entity_id,
        claim_entity_id: request.claim_entity_id,
        remaining_stock: request.stock,
        required_approval: request.required_approval,
        required_skill_id: request.required_skill_id,
        required_skill_level: request.required_skill_level,
    };

    ctx.db.claim_recruitment_state().try_insert(recruitment_offer)?;

    Ok(())
}
