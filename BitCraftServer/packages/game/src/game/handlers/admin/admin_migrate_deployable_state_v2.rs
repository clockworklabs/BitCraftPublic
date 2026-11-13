use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{deployable_state, deployable_state_v2, DeployableStateV2},
    },
};

#[spacetimedb::reducer]
pub fn admin_migrate_deployable_state_v2(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut copied = 0;
    let mut skipped = 0;

    for deployable in ctx.db.deployable_state().iter() {
        if ctx.db.deployable_state_v2().entity_id().find(deployable.entity_id).is_some() {
            skipped += 1;
            continue;
        }

        ctx.db.deployable_state_v2().insert(DeployableStateV2 {
            entity_id: deployable.entity_id,
            owner_id: deployable.owner_id,
            claim_entity_id: deployable.claim_entity_id,
            direction: deployable.direction,
            deployable_description_id: deployable.deployable_description_id,
            nickname: deployable.nickname,
            hidden: deployable.hidden,
            appearance_override_id: 0,
        });
        copied += 1;
    }

    log::info!("Copied {copied} deployable_state rows into deployable_state_v2");
    log::info!("Skipped {skipped} deployable_state rows already present in deployable_state_v2");

    Ok(())
}
