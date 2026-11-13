use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    inter_module::on_claim_members_changed,
    messages::{action_request::PlayerClaimRemoveMemberRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_remove_member(ctx: &ReducerContext, request: PlayerClaimRemoveMemberRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");

    let member_id = request.player_entity_id;

    let editor_member = unwrap_or_err!(claim.get_member(ctx, actor_id), "You are not a member of this claim");
    let editor_score = claim.score_permissions(&editor_member);

    let target_member = unwrap_or_err!(claim.get_member(ctx, member_id), "Target is not a member of this claim");
    let target_score = claim.score_permissions(&target_member);

    if editor_score <= target_score {
        return Err("You don't have the credentials to remove this member from the claim".into());
    }

    for rent_state in ctx.db.rent_state().claim_entity_id().filter(claim.entity_id) {
        if let Some(renter) = rent_state.white_list.first() {
            if *renter == member_id {
                return Err("Cannot remove a member renting a building. You need to evict them first.".into());
            }
        }
    }

    ClaimMemberState::delete_shared(ctx, target_member, crate::inter_module::InterModuleDestination::Global);
    on_claim_members_changed::send_message(ctx, request.claim_entity_id);

    Ok(())
}
