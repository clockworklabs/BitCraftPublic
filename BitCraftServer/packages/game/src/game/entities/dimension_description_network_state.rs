use spacetimedb::ReducerContext;

use crate::{dimension_description_state, dimension_network_state, messages::components::DimensionNetworkState};

impl DimensionNetworkState {
    pub fn get(ctx: &ReducerContext, dimension: u32) -> Option<DimensionNetworkState> {
        if let Some(dimension_description) = ctx.db.dimension_description_state().dimension_id().find(dimension) {
            return ctx
                .db
                .dimension_network_state()
                .entity_id()
                .find(dimension_description.dimension_network_entity_id);
        }
        None
    }
}
