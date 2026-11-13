use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::handlers::claim::claim_tech_unlock_tech;
use crate::{claim_tech_state, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
fn cheat_claim_totem_complete_current_research(ctx: &ReducerContext, claim_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimTotemCurrentResearch) {
        return Err("Unauthorized.".into());
    }

    let claim_tech = unwrap_or_err!(
        ctx.db.claim_tech_state().entity_id().find(claim_entity_id),
        "Claim has no tech, this should not happen"
    );

    // Make sure tech is being researched
    if let Some(_cancel_token) = claim_tech.scheduled_id {
        if claim_tech.researching == 0 {
            return Err("Not researching a tech.".into());
        }

        let tech_id = claim_tech.researching;

        claim_tech_unlock_tech::reduce(ctx, claim_entity_id, tech_id)
    } else {
        Err("Tech is not being researched".into())
    }
}
