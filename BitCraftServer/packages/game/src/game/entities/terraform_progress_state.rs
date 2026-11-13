use spacetimedb::ReducerContext;

use crate::{claim_state, messages::components::TerraformProgressState};

impl TerraformProgressState {
    pub fn validate_permission_set_final_elevation(&self, ctx: &ReducerContext, actor_id: u64, claim_entity_id: u64) -> Result<(), String> {
        //inside claims only players with build permission can set the final height target,
        if let Some(claim_entity) = ctx.db.claim_state().entity_id().find(claim_entity_id) {
            if !claim_entity.has_build_permissions(ctx, actor_id) {
                return Err("cannot set dig height without build permissions".into());
            }
        }

        Ok(())
    }

    pub fn validate_permission_terraform(&self, ctx: &ReducerContext, actor_id: u64, claim_entity_id: u64) -> Result<(), String> {
        //inside claims only players with build permission can set the final height target,
        if let Some(claim_entity) = ctx.db.claim_state().entity_id().find(claim_entity_id) {
            if !claim_entity.get_member(ctx, actor_id).is_some() {
                return Err("Non claim members cannot terraform".into());
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.progress = i32::MIN;
    }
}
