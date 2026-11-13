use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        handlers::empires::empires::{empire_craft_supplies_timer, EmpireCraftSuppliesTimer},
        reducer_helpers::timer_helpers::now_plus_secs,
    },
    messages::{empire_schema::empire_foundry_state, empire_shared::*, inter_module::*, static_data::parameters_desc},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireQueueSuppliesMsg) -> Result<(), String> {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    let empire_currency_cost = params.hexite_capsule_currency_cost;

    let mut empire = unwrap_or_err!(
        ctx.db
            .empire_state()
            .capital_building_entity_id()
            .find(request.claim_building_entity_id),
        "The foundry can only be used in the capital city"
    );

    if empire.empire_currency_treasury < empire_currency_cost {
        return Err("Insufficient treasury to craft a hexite capsule".into());
    }
    empire.empire_currency_treasury -= empire_currency_cost;

    EmpireState::update_shared(ctx, empire, crate::inter_module::InterModuleDestination::AllOtherRegions);

    let mut foundry = unwrap_or_err!(
        ctx.db.empire_foundry_state().entity_id().find(request.building_entity_id),
        "This is not an empire foundry"
    );
    foundry.queued += 1;
    if foundry.queued == 1 {
        // Start the craft
        foundry.started = ctx.timestamp;
        ctx.db.empire_craft_supplies_timer().insert(EmpireCraftSuppliesTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(params.hexite_capsule_craft_time_seconds as u64, ctx.timestamp),
            foundry_entity_id: request.building_entity_id,
        });
    }
    ctx.db.empire_foundry_state().entity_id().update(foundry);

    Ok(())
}
