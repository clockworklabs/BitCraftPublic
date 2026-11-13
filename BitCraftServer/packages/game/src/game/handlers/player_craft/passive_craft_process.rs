use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::handlers::authentication::has_role,
    messages::{authentication::Role, components::*},
};

#[spacetimedb::table(accessor = passive_craft_timer, scheduled(passive_craft_process, at = scheduled_at))]
pub struct PassiveCraftTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    #[unique]
    pub craft_entity_id: u64,
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn passive_craft_process(ctx: &ReducerContext, timer: PassiveCraftTimer) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    // Complete craft if it was not already cancelled
    if let Some(mut craft) = ctx.db.passive_craft_state().entity_id().find(timer.craft_entity_id) {
        let building_entity_id = craft.building_entity_id;
        let slot = craft.slot.unwrap();

        craft.timestamp = ctx.timestamp;
        craft.slot = None;
        craft.status = PassiveCraftStatus::Complete;
        ctx.db.passive_craft_state().entity_id().update(craft);
        PassiveCraftState::process_oldest_queued(ctx, building_entity_id, slot, ctx.timestamp);
    }
    Ok(())
}
