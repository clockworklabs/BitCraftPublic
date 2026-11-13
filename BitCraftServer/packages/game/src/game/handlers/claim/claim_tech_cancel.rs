use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Timestamp};

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimTechCancelRequest, components::*},
    unwrap_or_err,
};

use super::claim_tech_unlock_tech::claim_tech_unlock_timer;

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_tech_cancel(ctx: &ReducerContext, request: PlayerClaimTechCancelRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim_entity_id = request.claim_entity_id;

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "No such claim.");

    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Only the owner and co-owners can cancel ongoing claim technologies.".into());
    }

    let mut claim_tech = unwrap_or_err!(
        ctx.db.claim_tech_state().entity_id().find(claim_entity_id),
        "Claim has no tech, this should not happen"
    );

    // Make sure tech is being researched
    if let Some(scheduled_id) = claim_tech.scheduled_id {
        ctx.db.claim_tech_unlock_timer().scheduled_id().delete(scheduled_id);
        claim_tech.scheduled_id = None;
        claim_tech.researching = 0;
        claim_tech.start_timestamp = Timestamp::UNIX_EPOCH;
        ctx.db.claim_tech_state().entity_id().update(claim_tech);
    } else {
        return Err("Tech is not being researched".into());
    }

    Ok(())
}
