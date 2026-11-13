use bitcraft_macro::feature_gate;
use crate::game::game_state;
use crate::messages::action_request::PlayerClaimTransferOwnershipRequest;
use crate::messages::components::*;
use crate::unwrap_or_err;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_transfer_ownership(ctx: &ReducerContext, request: PlayerClaimTransferOwnershipRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");

    if !claim.has_owner_permissions(actor_id) {
        return Err("Only the owner can transfer claim ownership.".into());
    }

    let new_owner_id = request.new_owner_entity_id;
    let claim_member = ctx
        .db
        .claim_member_state()
        .player_claim()
        .filter((new_owner_id, request.claim_entity_id))
        .next();
    if claim.owner_player_entity_id != 0 && claim_member.is_none() {
        return Err("Player is not a member of this claim.".into());
    }

    let mut new_owner = claim_member.unwrap();
    new_owner.build_permission = true;
    new_owner.inventory_permission = true;
    new_owner.officer_permission = true;
    new_owner.co_owner_permission = true;
    ClaimMemberState::update_shared(ctx, new_owner, crate::inter_module::InterModuleDestination::Global);

    claim.owner_player_entity_id = new_owner_id;
    ClaimState::update_shared(ctx, claim, crate::inter_module::InterModuleDestination::Global);

    Ok(())
}

// NOTE: Keeping the code commented out in case we want to return to auto-transfer
/*
#[allow(dead_code)]
fn auto_transfer_previous_claim(ctx: &ReducerContext, actor_id: u64) {
    // if actor_id is already owning another claim, we need to transfer or abandon the previous claim
    if let Some(mut previous_claim) = ctx.db.claim_description_state().owner_player_entity_id().filter(actor_id).next() {
        // remove owner
        previous_claim.members.remove(0);
        previous_claim.owner_player_entity_id = 0;
        // find a new owner based on rank and seniority (officers -> builders -> inventory -> members)
        for j in 0..4 {
            for i in 0..previous_claim.members.len() {
                if previous_claim.owner_player_entity_id != 0 {
                    // a new owner was already found
                    break;
                }

                let member = &previous_claim.members[i];
                if j == 0 && !member.officer_permission {
                    // first pass: only officers
                    continue;
                }
                if j == 1 && (member.officer_permission || !member.build_permission) {
                    // second pass: only builders except officers
                    continue;
                }
                if j == 2 && (member.officer_permission || member.build_permission || !member.inventory_permission) {
                    // third pass: inventory members except officers and builders
                    continue;
                }
                if j == 3 && (member.officer_permission || member.build_permission || member.inventory_permission) {
                    // final pass: left-overs
                    continue;
                }
                if !PlayerState::owns_claim(ctx, member.player_entity_id) {
                    // appoint new owner
                    let mut member = previous_claim.members.remove(i);
                    member.build_permission = true;
                    member.inventory_permission = true;
                    member.officer_permission = true;
                    previous_claim.owner_player_entity_id = member.player_entity_id;
                    previous_claim.members.insert(0, member);
                    break;
                }
            }
        }
        ctx.db.claim_description_state().entity_id().update(previous_claim);
    }
}*/
