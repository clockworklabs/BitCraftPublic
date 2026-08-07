use bitcraft_macro::feature_gate;
use crate::game::reducer_helpers::footprint_helpers;
use crate::game::entities::growth_timer::delete_resource_growth_timer;
use crate::game::{game_state, permission_helper};
use crate::messages::action_request::PlayerProjectSiteCancelRequest;
use crate::messages::components::*;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn project_site_cancel(ctx: &ReducerContext, request: PlayerProjectSiteCancelRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let location = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(&request.owner_entity_id),
        "Invalid project site"
    );

    let coordinates = location.coordinates();

    if !PermissionState::can_interact_with_tile(ctx, actor_id, coordinates, Permission::Build) {
        return Err("You don't have permission to cancel this project site".into());
    }

    if !permission_helper::can_interact_with_tile(ctx, coordinates, actor_id, ClaimPermission::Build) {
        return Err("You don't have permission to cancel this project site".into());
    }

    footprint_helpers::delete_footprint(ctx, request.owner_entity_id);
    ctx.db.location_state().entity_id().delete(&request.owner_entity_id);
    ctx.db.project_site_state().entity_id().delete(&request.owner_entity_id);
    ctx.db.auto_claim_state().entity_id().delete(&request.owner_entity_id);
    ctx.db.growth_state().entity_id().delete(&request.owner_entity_id);
    delete_resource_growth_timer(ctx, request.owner_entity_id);

    Ok(())
}
