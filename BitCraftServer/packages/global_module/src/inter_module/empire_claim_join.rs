use spacetimedb::ReducerContext;

use crate::{
    messages::{empire_schema::*, empire_shared::*, inter_module::*, static_data::EmpireNotificationType},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireClaimJoinMsg) -> Result<(), String> {
    let claim_entity_id = request.claim_entity_id;
    let building_entity_id = request.claim_building_entity_id;
    let empire_entity_id = request.empire_entity_id;

    let mut empire = unwrap_or_err!(ctx.db.empire_state().entity_id().find(empire_entity_id), "Empire does not exist");

    let mut settlement = unwrap_or_err!(
        ctx.db.empire_settlement_state().building_entity_id().find(building_entity_id),
        "This claim does not have the tech to join an empire"
    );

    if settlement.empire_entity_id != 0 {
        if settlement.empire_entity_id == empire_entity_id {
            return Err("Already part of this empire".into());
        }
        return Err("Already part of another empire".into());
    }
    settlement.empire_entity_id = empire_entity_id;
    EmpireSettlementState::update_shared(ctx, settlement, super::InterModuleDestination::AllOtherRegions);

    // This claim might start with donations if it already has empire citizens
    EmpireSettlementState::update_donations(ctx, claim_entity_id, 0)?;

    empire.num_claims += 1;
    EmpireState::update_shared(ctx, empire, crate::inter_module::InterModuleDestination::AllOtherRegions);

    // Claim Joined Notification (11)
    EmpireNotificationState::new(ctx, EmpireNotificationType::ClaimJoined, empire_entity_id, vec![request.claim_name]);

    Ok(())
}
