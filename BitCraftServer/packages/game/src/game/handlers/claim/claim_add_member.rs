use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimAddMemberRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_add_member(ctx: &ReducerContext, request: PlayerClaimAddMemberRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    // validate permissions for non-internal queries
    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");

    if !claim.has_officer_permissions(ctx, actor_id) {
        return Err("Only the owner, co-owners and officers can add new members.".into());
    }

    // validate player by name
    let lowercase_name = request.player_name.to_lowercase();
    let player = ctx.db.player_lowercase_username_state().username_lowercase().find(&lowercase_name);
    if player.is_none() {
        return Err("Player does not exist.".into());
    }

    reduce(ctx, claim, request.claim_entity_id, player.unwrap().entity_id)
}

pub fn reduce(ctx: &ReducerContext, claim: ClaimState, claim_entity_id: u64, player_entity_id: u64) -> Result<(), String> {
    if claim.get_member(ctx, player_entity_id).is_some() {
        return Err("Already member of this claim.".into());
    }

    if ctx.db.claim_member_state().claim_entity_id().filter(claim.entity_id).count()
        >= ctx
            .db
            .claim_tech_state()
            .entity_id()
            .find(claim_entity_id)
            .unwrap()
            .max_members(ctx) as usize
    {
        return Err("Already at maximum amount of members. Upgrade your claim tech for more.".into());
    }

    claim.add_member(ctx, player_entity_id, false, false, false, false)?;

    Ok(())
}
