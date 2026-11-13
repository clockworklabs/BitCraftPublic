use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{game::game_state, messages::components::*, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_set_protection_threshold(ctx: &ReducerContext, building_entity_id: u64, hours: u32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if hours > 2400 {
        return Err("Invalid protection threshold value".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(building_entity_id),
        "No such building to repair."
    );

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(building.claim_entity_id), "No such claim.");

    if !claim.has_owner_permissions(actor_id) {
        return Err("Only the owner can set those values.".into());
    }

    if let Some(mut protection_threshold) = ctx
        .db
        .claim_local_supply_security_threshold_state()
        .entity_id()
        .find(building.claim_entity_id)
    {
        protection_threshold.supply_security_threshold_hours = hours as i32;
        ctx.db
            .claim_local_supply_security_threshold_state()
            .entity_id()
            .update(protection_threshold);
    } else {
        let protection_threshold = ClaimLocalSupplySecurityThresholdState {
            entity_id: building.claim_entity_id,
            supply_security_threshold_hours: hours as i32,
        };
        ctx.db.claim_local_supply_security_threshold_state().insert(protection_threshold);
    }

    Ok(())
}
