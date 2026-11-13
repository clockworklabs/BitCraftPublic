use spacetimedb::ReducerContext;

use super::project_site_state::SmallHexTile;
use crate::messages::components::DeployableStateV2;
use crate::{deployable_desc, deployable_state_v2, mobile_entity_state, mounting_state};

impl DeployableStateV2 {
    pub fn coordinates(ctx: &ReducerContext, deployable_entity_id: u64) -> SmallHexTile {
        ctx.db
            .mobile_entity_state()
            .entity_id()
            .find(deployable_entity_id)
            .unwrap()
            .coordinates()
    }

    // DAB Note: was not able to conciliate the new spacetimedb signature with the lifetimes etc.
    pub fn passengers_iter(ctx: &ReducerContext, deployable_entity_id: u64) -> Vec<u64> {
        // Find all entities with mount components referencing this deployable
        let passengers: Vec<u64> = ctx
            .db
            .mounting_state()
            .deployable_entity_id()
            .filter(&deployable_entity_id)
            .map(|m| m.entity_id)
            .collect();
        passengers
    }

    pub fn free_slots(ctx: &ReducerContext, deployable_entity_id: u64) -> Vec<i32> {
        let used_slots: Vec<i32> = ctx
            .db
            .mounting_state()
            .deployable_entity_id()
            .filter(deployable_entity_id)
            .map(|m| m.deployable_slot)
            .collect();
        let deployable = ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id).unwrap();
        let deployable_desc = ctx
            .db
            .deployable_desc()
            .id()
            .find(deployable.deployable_description_id)
            .unwrap();
        let mut slots = Vec::new();
        for i in 0..deployable_desc.capacity {
            if !used_slots.contains(&i) {
                slots.push(i);
            }
        }
        slots
    }
}
