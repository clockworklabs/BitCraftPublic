use spacetimedb::ReducerContext;

use crate::{location_state, mobile_entity_state, TargetableState};

use super::project_site_state::SmallHexTile;

impl TargetableState {
    pub fn new(entity_id: u64) -> TargetableState {
        TargetableState { entity_id }
    }

    pub fn coordinates(&self, ctx: &ReducerContext) -> SmallHexTile {
        match ctx.db.mobile_entity_state().entity_id().find(self.entity_id) {
            Some(location) => location.coordinates(),
            None => match ctx.db.location_state().entity_id().find(self.entity_id) {
                Some(location) => location.coordinates(),
                None => SmallHexTile { x: 0, z: 0, dimension: 0 },
            },
        }
    }
}
