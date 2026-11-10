use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{self, game_state_filters};
use crate::game::reducer_helpers::timer_helpers::now_plus_secs;
use crate::{claim_tech_desc, parameters_desc, params, InventoryState};
use crate::{
    messages::{action_request::PlayerClaimTechLearnRequest, components::*},
    unwrap_or_err,
};

use super::claim_tech_unlock_tech::{claim_tech_unlock_timer, ClaimTechUnlockTimer};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_tech_learn(ctx: &ReducerContext, request: PlayerClaimTechLearnRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim_entity_id = request.claim_entity_id;
    let tech_id = request.tech_id;

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "No such claim.");
    let claim_building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(claim.owner_building_entity_id),
        "Could not find settlement"
    );

    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);
    if claim_building.distance_to(ctx, &player_coord) > 2 {
        return Err("Too far".into());
    }

    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Only the owner and co-owners can research claim technologies.".into());
    }

    // Make sure requisite is known
    let tech_desc = unwrap_or_err!(ctx.db.claim_tech_desc().id().find(tech_id), "Tech does not exist");
    if tech_desc.cannot_be_researched {
        return Err("This technology cannot be researched directly.".into());
    }
    let mut claim_tech = unwrap_or_err!(
        ctx.db.claim_tech_state().entity_id().find(&claim_entity_id),
        "Claim has no tech, this should not happen"
    );

    if tech_desc
        .blocking_claim_techs
        .as_ref()
        .is_some_and(|blocking_techs| blocking_techs.iter().any(|tech| claim_tech.learned.contains(tech)))
    {
        return Err("A technology that blocks this research has already been learned.".into());
    }

    for req in tech_desc.requirements {
        if !claim_tech.learned.contains(&req) {
            return Err("Missing required tech.".into());
        }
    }

    // make sure the player has the required items
    if tech_desc.input.len() > 0 {
        if !InventoryState::remove_stacks_from_player_inventory(ctx, actor_id, &tech_desc.input, true) {
            return Err("Missing required items".into());
        }
    }

    // Make sure no research is in progress
    if claim_tech.researching != 0 {
        return Err("Already researching a tech.".into());
    }

    if claim_tech.learned.contains(&tech_id) {
        return Err("Already known.".into());
    }

    // Expend supplies
    let claim_local = claim.local_state(ctx);

    let threshold_hours = match ctx
        .db
        .claim_local_supply_security_threshold_state()
        .entity_id()
        .find(claim_entity_id)
    {
        Some(ts) => ts.supply_security_threshold_hours,
        None => params!(ctx).co_owner_take_ownership_supply_time / 3600,
    };
    let threshold = (threshold_hours as f32 * claim_local.full_maintenance(ctx)) as i32;

    if claim_local.supplies - tech_desc.supplies_cost < threshold {
        return Err("Insufficient supplies.".into());
    }

    claim_local.update_supplies_and_commit(ctx, -tech_desc.supplies_cost as f32, true)?;

    // Timer to complete tech
    let timer = ctx.db.claim_tech_unlock_timer().try_insert(ClaimTechUnlockTimer {
        scheduled_at: now_plus_secs(tech_desc.research_time as u64, ctx.timestamp),
        scheduled_id: 0,
        claim_entity_id,
        tech_id,
    })?;

    claim_tech.scheduled_id = Some(timer.scheduled_id);
    claim_tech.researching = tech_id;
    claim_tech.start_timestamp = ctx.timestamp;
    ctx.db.claim_tech_state().entity_id().update(claim_tech);

    Ok(())
}
