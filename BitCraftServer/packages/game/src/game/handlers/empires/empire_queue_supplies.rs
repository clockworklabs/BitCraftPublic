use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    inter_module::*,
    messages::{components::*, empire_shared::*, inter_module::*, static_data::parameters_desc},
    params, unwrap_or_err, unwrap_or_return,
};

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_queue_supplies(ctx: &ReducerContext, request: EmpireQueueSuppliesRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::CraftHexiteCapsule) {
        return Err("You don't have the permissions to craft a hexite capsule".into());
    }

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&request.building_entity_id),
        "Invalid building"
    );

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(&building.claim_entity_id), "Not claimed");
    let mut claim_local = claim.local_state(ctx);

    let empire = unwrap_or_err!(
        ctx.db
            .empire_state()
            .capital_building_entity_id()
            .find(&claim.owner_building_entity_id),
        "The foundry can only be used in the capital city"
    );

    let params = ctx.db.parameters_desc().version().find(&0).unwrap();
    let empire_currency_cost = params.hexite_capsule_shard_cost as u32;
    let supplies_cost = params.hexite_capsule_supply_cost;
    let threshold_hours = match ctx
        .db
        .claim_local_supply_security_threshold_state()
        .entity_id()
        .find(building.claim_entity_id)
    {
        Some(ts) => ts.supply_security_threshold_hours,
        None => params!(ctx).co_owner_take_ownership_supply_time / 3600,
    };
    let threshold = (threshold_hours as f32 * claim_local.full_maintenance(ctx)) as i32;

    if claim_local.supplies - supplies_cost < threshold {
        return Err("You don't have enough claim supplies to craft a hexite capsule".into());
    }

    if empire.empire_currency_treasury < empire_currency_cost {
        return Err("You don't have enough in treasury to craft a hexite capsule".into());
    }

    claim_local.supplies -= supplies_cost;
    ctx.db.claim_local_state().entity_id().update(claim_local);

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireQueueSupplies(EmpireQueueSuppliesMsg {
            player_entity_id: actor_id,
            building_entity_id: request.building_entity_id,
            claim_entity_id: claim.entity_id,
            claim_building_entity_id: claim.owner_building_entity_id,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireQueueSuppliesMsg, error: Option<String>) {
    if error.is_some() {
        //Refund supplies if remote call fails
        let mut claim = unwrap_or_return!(ctx.db.claim_local_state().entity_id().find(&request.claim_entity_id), "Not claimed");
        claim.supplies += params!(ctx).hexite_capsule_supply_cost;
        ctx.db.claim_local_state().entity_id().update(claim);

        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
