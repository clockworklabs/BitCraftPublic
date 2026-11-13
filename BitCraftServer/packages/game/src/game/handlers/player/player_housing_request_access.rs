use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::{action_request::PlayerHousingRequestAccessRequest, components::*},
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_housing_request_access(ctx: &ReducerContext, request: PlayerHousingRequestAccessRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let (player_housing, _building) =
        PlayerHousingState::get_and_validate_player_housing(ctx, actor_id, request.building_entity_id, true, request.owner_entity_id)?;

    if PermissionState::get(ctx, player_housing.entity_id, actor_id).is_some() {
        return Err("You already have a permission set for this house".into());
    }

    let permission = PermissionState::new(
        ctx,
        request.owner_entity_id,
        actor_id,
        PermissionGroup::Player,
        Permission::PendingVisitor,
    )?;
    ctx.db.permission_state().insert(permission);

    Ok(())
}
