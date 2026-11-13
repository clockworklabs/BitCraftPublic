use bitcraft_macro::feature_gate;
use crate::messages::static_data::parameters_desc;
use crate::{
    game::game_state,
    messages::{action_request::PlayerClaimTakeOwnershipRequest, components::*},
    params, unwrap_or_err,
};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_take_ownership(ctx: &ReducerContext, request: PlayerClaimTakeOwnershipRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.claim_entity_id)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, claim_entity_id: u64) -> Result<(), String> {
    let mut claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "No such claim.");
    let building_coord = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(claim.owner_building_entity_id),
        "Invalid claim"
    )
    .coordinates();
    let player_coord = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Invalid player").coordinates();

    if building_coord.distance_to(player_coord) > 3 {
        return Err("Too far".into());
    }

    if ctx.db.claim_member_state().claim_entity_id().filter(&claim_entity_id).count() == 0 {
        add_player_to_claim(ctx, actor_id, &mut claim)?;
    } else {
        let local_claim = claim.local_state(ctx);
        let claim_supplies = local_claim.supplies;
        if claim_supplies <= 0 {
            match claim.get_member(ctx, actor_id) {
                Some(value) => value.set_permissions(ctx, true, true, true, true),
                None => add_player_to_claim(ctx, actor_id, &mut claim)?,
            }
        } else {
            let existing_member = unwrap_or_err!(
                claim.get_member(ctx, actor_id),
                "Only claim members can take ownership when the claim is supplied"
            );

            let supply_time_secs = if existing_member.co_owner_permission {
                params!(ctx).co_owner_take_ownership_supply_time
            } else if existing_member.officer_permission {
                params!(ctx).officer_take_ownership_supply_time
            } else {
                params!(ctx).member_take_ownership_supply_time
            };

            log::info!(
                "supply_time_secs => {supply_time_secs} claim_supplies = {claim_supplies} required = {}",
                local_claim.get_required_supplies_for_seconds(ctx, supply_time_secs)
            );
            if claim_supplies >= local_claim.get_required_supplies_for_seconds(ctx, supply_time_secs) {
                return Err("You cannot take ownership yet".into());
            }
            existing_member.set_permissions(ctx, true, true, true, true);
        }
    }

    claim.owner_player_entity_id = actor_id;
    ClaimState::update_shared(ctx, claim, crate::inter_module::InterModuleDestination::Global);

    Ok(())
}

fn add_player_to_claim(ctx: &ReducerContext, actor_id: u64, claim: &mut ClaimState) -> Result<(), String> {
    claim.add_member(ctx, actor_id, true, true, true, true)
}
