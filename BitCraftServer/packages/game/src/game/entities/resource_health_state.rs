use spacetimedb::ReducerContext;

use crate::{messages::components::ResourceHealthState, resource_health_state};

impl ResourceHealthState {
    pub fn is_depleted(ctx: &ReducerContext, entity_id: u64) -> bool {
        if let Some(health) = ctx.db.resource_health_state().entity_id().find(entity_id) {
            health.health <= 0
        } else {
            false
        }
    }

    pub fn is_depleted_self(&self) -> bool {
        self.health <= 0
    }
}
