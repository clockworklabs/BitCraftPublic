use spacetimedb::ReducerContext;

use crate::{game::reducer_helpers::deployable_helpers::store_deployable, messages::components::*};

use super::cheat_type::{can_run_cheat, CheatType};

#[spacetimedb::reducer]
pub fn cheat_deployable_store(ctx: &ReducerContext, deployable_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDeployableStore) {
        return Err("Unauthorized.".into());
    }

    if ctx.db.mobile_entity_state().entity_id().find(deployable_entity_id).is_none() {
        return Err("Deployable is already stored".into());
    }

    if let Some(deployable_state) = ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id) {
        let actor_id = deployable_state.owner_id;
        return store_deployable(ctx, actor_id, deployable_entity_id, false);
    } else if let Some(mut deployable_collectible) = ctx
        .db
        .deployable_collectible_state()
        .deployable_entity_id()
        .find(deployable_entity_id)
    {
        if deployable_collectible.location.is_some() {
            deployable_collectible.location = None;
            ctx.db
                .deployable_collectible_state()
                .deployable_entity_id()
                .update(deployable_collectible);
        }
        return Ok(());
    }
    Err("Deployable not found".into())
}
