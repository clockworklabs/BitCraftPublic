use spacetimedb::ReducerContext;

use crate::{
    messages::{empire_schema::*, empire_shared::*, inter_module::*},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireCollectHexiteCapsuleMsg) -> Result<(), String> {
    let mut foundry = unwrap_or_err!(
        ctx.db.empire_foundry_state().entity_id().find(request.building_entity_id),
        "This is not an empire foundry"
    );

    if !EmpirePlayerDataState::has_permission(ctx, request.player_entity_id, EmpirePermission::CollectHexiteCapsule) {
        return Err("You don't have the permissions to collect a hexite capsule".into());
    }

    if foundry.hexite_capsules == 0 {
        return Err("There are no hexite capsule in this foundry to collect".into());
    }

    foundry.hexite_capsules -= 1;
    ctx.db.empire_foundry_state().entity_id().update(foundry);

    Ok(())
}
