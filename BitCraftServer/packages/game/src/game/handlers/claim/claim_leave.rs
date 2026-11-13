use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    inter_module::on_claim_members_changed,
    messages::{action_request::PlayerClaimLeaveRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_leave(ctx: &ReducerContext, request: PlayerClaimLeaveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");

    let member = unwrap_or_err!(claim.get_member(ctx, actor_id), "You're not a member of this claim.");

    for rent_state in ctx.db.rent_state().claim_entity_id().filter(claim.entity_id) {
        if let Some(renter) = rent_state.white_list.first() {
            if *renter == actor_id {
                return Err("Cannot leave a claim while renting a building.".into());
            }
        }
    }

    if claim.has_owner_permissions(actor_id) {
        claim.owner_player_entity_id = 0;
        ClaimState::update_shared(ctx, claim, crate::inter_module::InterModuleDestination::Global);
    }

    ClaimMemberState::delete_shared(ctx, member, crate::inter_module::InterModuleDestination::Global);
    on_claim_members_changed::send_message(ctx, request.claim_entity_id);

    Ok(())
}
