use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimSetMemberPermissionsRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_set_member_permissions(ctx: &ReducerContext, request: PlayerClaimSetMemberPermissionsRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(request.claim_entity_id),
        "No such claim."
    );

    let member_id = request.player_entity_id;

    // Check if target is the owner.
    if claim.has_owner_permissions(member_id) {
        return Err("Can't edit owner's permissions.".into());
    }

    if (request.co_owner && !claim.has_owner_permissions(actor_id))
        || (request.officer && !claim.has_co_owner_permissions(ctx, actor_id))
        || !claim.has_officer_permissions(ctx, actor_id)
    {
        return Err("You don't have the credentials to bestow those permissions.".into());
    }

    let editor_member = unwrap_or_err!(claim.get_member(ctx, actor_id), "You are not a member of this claim");
    let editor_score = claim.score_permissions(&editor_member);

    let mut target_member = unwrap_or_err!(claim.get_member(ctx, member_id), "Target is not a member of this claim"); // ...can't get a mut member here because of #rust...
    let target_score = claim.score_permissions(&target_member);

    // Make sure we're not editing someone of equal or higher rank
    if editor_score <= target_score {
        return Err("You don't have the credentials to change that member's permissions.".into());
    }

    target_member.inventory_permission = request.inventory | request.officer | request.co_owner;
    target_member.build_permission = request.build | request.officer | request.co_owner;
    target_member.officer_permission = request.officer | request.co_owner;
    target_member.co_owner_permission = request.co_owner;
    ctx.db.claim_member_state().entity_id().update(target_member);

    Ok(())
}
