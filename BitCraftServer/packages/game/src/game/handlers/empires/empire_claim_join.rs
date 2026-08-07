use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    inter_module::*,
    messages::{components::*, empire_shared::*, inter_module::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_claim_join(ctx: &ReducerContext, building_entity_id: u64, empire_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if ctx.db.empire_state().entity_id().find(&empire_entity_id).is_none() {
        return Err("Empire does not exist".into());
    }

    let claim = unwrap_or_err!(
        ctx.db.claim_state().owner_building_entity_id().find(&building_entity_id),
        "Claim is not valid"
    );

    if claim.owner_player_entity_id != actor_id {
        return Err("Only the claim owner can decide to join an empire".into());
    }

    let settlement = unwrap_or_err!(
        ctx.db.empire_settlement_state().building_entity_id().find(&building_entity_id),
        "This claim does not have the tech to join an empire"
    );

    if settlement.empire_entity_id != 0 {
        if settlement.empire_entity_id == empire_entity_id {
            return Err("Already part of this empire".into());
        }
        return Err("Already part of another empire".into());
    }

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireClaimJoin(EmpireClaimJoinMsg {
            player_entity_id: actor_id,
            claim_entity_id: claim.entity_id,
            claim_building_entity_id: building_entity_id,
            empire_entity_id,
            claim_name: claim.name,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireClaimJoinMsg, error: Option<String>) {
    if error.is_some() {
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
