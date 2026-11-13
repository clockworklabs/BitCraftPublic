use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Timestamp};

use crate::{
    claim_state, claim_tech_state,
    inter_module::claim_create_empire_settlement_state,
    messages::{authentication::ServerIdentity, components::ClaimTechState, static_data::claim_tech_desc},
    unwrap_or_err, unwrap_or_return,
};

#[spacetimedb::table(accessor = claim_tech_unlock_timer, scheduled(claim_tech_unlock_tech, at = scheduled_at))]
pub struct ClaimTechUnlockTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub claim_entity_id: u64,
    pub tech_id: i32,
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_tech_unlock_tech(ctx: &ReducerContext, timer: ClaimTechUnlockTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    self::reduce(ctx, timer.claim_entity_id, timer.tech_id)
}

pub fn reduce(ctx: &ReducerContext, claim_entity_id: u64, tech_id: i32) -> Result<(), String> {
    // Make sure no research is in progress
    let mut claim_tech = unwrap_or_err!(
        ctx.db.claim_tech_state().entity_id().find(claim_entity_id),
        "Claim has no tech, this should not happen"
    );
    if claim_tech.researching == 0 {
        return Err("Not researching a tech.".into());
    }

    if claim_tech.learned.contains(&tech_id) {
        return Err("Already known.".into());
    }

    claim_tech.researching = 0;
    claim_tech.start_timestamp = Timestamp::UNIX_EPOCH;
    claim_tech.scheduled_id = None;
    unlock_claim_tech(ctx, &mut claim_tech, tech_id, true);
    ctx.db.claim_tech_state().entity_id().update(claim_tech);

    Ok(())
}

pub fn unlock_claim_tech(ctx: &ReducerContext, claim_tech: &mut ClaimTechState, tech_id: i32, ignore_if_already_learned: bool) {
    let already_learned = claim_tech.learned.contains(&tech_id);
    if ignore_if_already_learned && already_learned {
        return;
    }

    if !already_learned {
        claim_tech.learned.push(tech_id);

        // Hard coded: EMPIRE INFRASTRUCTURE
        if tech_id == 10000 {
            let claim = ctx.db.claim_state().entity_id().find(claim_tech.entity_id).unwrap();
            claim_create_empire_settlement_state::send_message(
                ctx,
                claim.entity_id,
                claim.owner_building_entity_id,
                claim.local_state(ctx).location.unwrap(),
            );
        }
    }

    let tech_desc = unwrap_or_return!(ctx.db.claim_tech_desc().id().find(tech_id), "Claim tech id {tech_id} doesn't exist");
    for unlocked_tech in &tech_desc.unlocks_techs {
        unlock_claim_tech(ctx, claim_tech, *unlocked_tech, ignore_if_already_learned);
    }
}
