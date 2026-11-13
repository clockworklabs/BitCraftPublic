use spacetimedb::{ReducerContext, Table, Timestamp};

use crate::{
    crafting_recipe_desc,
    game::{
        handlers::player_craft::passive_craft_process::{passive_craft_timer, PassiveCraftTimer},
        reducer_helpers::timer_helpers::now_plus_secs_f32,
    },
    passive_craft_state, PassiveCraftState, PassiveCraftStatus,
};

impl PassiveCraftState {
    pub fn process_oldest_queued(ctx: &ReducerContext, building_entity_id: u64, slot: u32, timestamp: Timestamp) {
        if let Some(mut oldest_queued_craft) = ctx
            .db
            .passive_craft_state()
            .building_entity_id()
            .filter(building_entity_id)
            .filter(|p| p.status == PassiveCraftStatus::Queued)
            .min_by_key(|p| p.timestamp)
        {
            let duration = ctx
                .db
                .crafting_recipe_desc()
                .id()
                .find(oldest_queued_craft.recipe_id)
                .unwrap()
                .time_requirement;

            oldest_queued_craft.timestamp = timestamp;
            oldest_queued_craft.slot = Some(slot);
            oldest_queued_craft.status = PassiveCraftStatus::Processing;
            let oldest_queued_craft_entity_id = oldest_queued_craft.entity_id;
            ctx.db.passive_craft_state().entity_id().update(oldest_queued_craft);

            ctx.db
                .passive_craft_timer()
                .try_insert(PassiveCraftTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(duration, ctx.timestamp),
                    craft_entity_id: oldest_queued_craft_entity_id,
                })
                .ok()
                .unwrap();
        }
    }
}
