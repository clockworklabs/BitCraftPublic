use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::handlers::claim::claim_tech_unlock_tech::claim_tech_unlock_timer;
use crate::inter_module::claim_create_empire_settlement_state;
use crate::{claim_state, claim_tech_desc, claim_tech_state, unwrap_or_err};
use spacetimedb::{ReducerContext, Table, Timestamp};

#[spacetimedb::reducer]
fn cheat_claim_totem_add_supplies(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    amount: f32, /* can be negative */
) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimTotemAddSupplies) {
        return Err("Unauthorized.".into());
    }

    let claim_desc = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "Claim doesn't exist");
    let _ = claim_desc.local_state(ctx).update_supplies_and_commit(ctx, amount, false);

    Ok(())
}

#[spacetimedb::reducer]
fn cheat_claim_totem_research_all(ctx: &ReducerContext, claim_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimTotemResearchAll) {
        return Err("Unauthorized.".into());
    }

    let mut tech = unwrap_or_err!(ctx.db.claim_tech_state().entity_id().find(claim_entity_id), "Claim doesn't exist");
    if let Some(id) = tech.scheduled_id {
        ctx.db.claim_tech_unlock_timer().scheduled_id().delete(id);
        tech.researching = 0;
        tech.start_timestamp = Timestamp::UNIX_EPOCH;
        tech.scheduled_id = None;
    }

    tech.learned = ctx.db.claim_tech_desc().iter().map(|a| a.id).collect();
    ctx.db.claim_tech_state().entity_id().update(tech);

    // EMPIRE INFRASTRUCTURE
    let claim = ctx.db.claim_state().entity_id().find(claim_entity_id).unwrap();
    claim_create_empire_settlement_state::send_message(
        ctx,
        claim.entity_id,
        claim.owner_building_entity_id,
        claim.local_state(ctx).location.unwrap(),
    );

    Ok(())
}

#[spacetimedb::reducer]
fn cheat_claim_unlock_tech(ctx: &ReducerContext, claim_entity_id: u64, tech_id: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimTotemUnlockTech) {
        return Err("Unauthorized.".into());
    }

    let mut tech = unwrap_or_err!(ctx.db.claim_tech_state().entity_id().find(claim_entity_id), "Claim doesn't exist");
    if let Some(id) = tech.scheduled_id {
        if let Some(timer) = ctx.db.claim_tech_unlock_timer().scheduled_id().find(id) {
            if timer.tech_id == tech_id {
                spacetimedb::log::info!("12");
                ctx.db.claim_tech_unlock_timer().scheduled_id().delete(id);
                tech.researching = 0;
                tech.start_timestamp = Timestamp::UNIX_EPOCH;
                tech.scheduled_id = None;
            }
        }
    }

    crate::game::handlers::claim::claim_tech_unlock_tech::unlock_claim_tech(ctx, &mut tech, tech_id, false);
    ctx.db.claim_tech_state().entity_id().update(tech);

    Ok(())
}

#[spacetimedb::reducer]
pub fn cheat_claims_complete_all_current_research(ctx: &ReducerContext) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimsCompleteAllCurrentResearch) {
        return Err("Unauthorized.".into());
    }

    for mut tech in ctx.db.claim_tech_state().iter() {
        if let Some(id) = tech.scheduled_id {
            if let Some(timer) = ctx.db.claim_tech_unlock_timer().scheduled_id().find(id) {
                ctx.db.claim_tech_unlock_timer().scheduled_id().delete(id);
                tech.researching = 0;
                tech.start_timestamp = Timestamp::UNIX_EPOCH;
                tech.scheduled_id = None;

                crate::game::handlers::claim::claim_tech_unlock_tech::unlock_claim_tech(ctx, &mut tech, timer.tech_id, false);
                ctx.db.claim_tech_state().entity_id().update(tech);
            }
        }
    }

    Ok(())
}
