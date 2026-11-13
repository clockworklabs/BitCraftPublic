use std::time::Duration;

use spacetimedb::{ReducerContext, TimeDuration, Timestamp};

use crate::{
    crafting_recipe_desc, game::game_state, parameters_desc, progressive_action_state, BuildingDesc, CraftingRecipeDesc,
    ProgressiveActionState, ProgressiveActionStatus,
};

impl ProgressiveActionState {
    pub fn new(
        ctx: &ReducerContext,
        owner_entity_id: u64,
        building_entity_id: u64,
        building_desc: &BuildingDesc,
        recipe_id: i32,
        craft_count: i32,
    ) -> ProgressiveActionState {
        let mut function_type = -1;

        // Find the function index. This might break if we start having crafting buildings with multiple crafting functions.
        for function in &building_desc.functions {
            if function.crafting_slots > 0 {
                function_type = function.function_type;
                break;
            }
        }

        let mut action = ProgressiveActionState {
            entity_id: game_state::create_entity(ctx),
            function_type,
            progress: 0,
            craft_count,
            recipe_id,
            building_entity_id,
            last_crit_outcome: 0,
            owner_entity_id,
            lock_expiration: Timestamp::UNIX_EPOCH,
            preparation: false,
        };
        action.set_expiration(ctx);
        action
    }

    pub fn get_status(&self, ctx: &ReducerContext) -> ProgressiveActionStatus {
        let recipe = ctx.db.crafting_recipe_desc().id().find(self.recipe_id).unwrap();
        self.get_status_from_recipe(&recipe, ctx.timestamp)
    }

    pub fn get_status_from_recipe(&self, recipe: &CraftingRecipeDesc, now: Timestamp) -> ProgressiveActionStatus {
        if self.progress >= recipe.actions_required * self.craft_count {
            return ProgressiveActionStatus::Completed;
        }

        if self.is_expired(now) {
            return ProgressiveActionStatus::Suspended;
        }

        ProgressiveActionStatus::Active
    }

    pub fn is_expired(&self, now: Timestamp) -> bool {
        now >= self.lock_expiration
    }

    pub fn set_expiration(&mut self, ctx: &ReducerContext) {
        self.lock_expiration = ctx.timestamp
            + TimeDuration::from(Duration::from_secs(
                ctx.db.parameters_desc().version().find(0).unwrap().crafting_lock_duration_secs as u64,
            ));
    }

    pub fn get_active_locks_on_building(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
    ) -> impl Iterator<Item = ProgressiveActionState> {
        let timestamp = ctx.timestamp;
        ctx.db
            .progressive_action_state()
            .owner_entity_id()
            .filter(actor_id)
            .filter(move |action| action.building_entity_id == building_entity_id && !action.is_expired(timestamp))
    }

    pub fn get_completed_crafts(&self, actions_required: i32) -> i32 {
        self.progress / actions_required
    }
    pub fn get_refunded_crafts(&self, actions_required: i32) -> i32 {
        0.max((self.craft_count * actions_required) - self.progress) / actions_required
    }
}
