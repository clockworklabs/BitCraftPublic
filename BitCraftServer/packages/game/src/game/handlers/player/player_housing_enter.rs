use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self, game_state_filters},
    messages::{action_request::PlayerHousingEnterRequest, components::*},
    unwrap_or_err,
};
use crate::messages::events::*;

use super::player_housing_update::player_housing_update_impl;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn player_housing_enter(ctx: &ReducerContext, request: PlayerHousingEnterRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    // If the owner enters his house, check for upgrades
    if actor_id == request.owner_entity_id {
        player_housing_update_impl(ctx, request.building_entity_id)?;
    }

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let (player_housing, _building) =
        PlayerHousingState::get_and_validate_player_housing(ctx, actor_id, request.building_entity_id, true, request.owner_entity_id)?;

    if let Some(permission) = PermissionState::get_permission_with_entity(ctx, actor_id, player_housing.entity_id, None, None) {
        if !permission.meets(Permission::Visitor) {
            return Err("You are not authorized to visit this house".into());
        }
    } else {
        return Err("You are not allowed to visit this house".into());
    }

    if ctx.timestamp < player_housing.locked_until {
        return Err("This house is locked and cannot be entered at the moment".into());
    }

    let location = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(player_housing.exit_portal_entity_id),
        "Cannot find the house entry point"
    );
    game_state_filters::teleport_to(ctx, actor_id, location.coordinates().into(), false, 0.0)?;
    ctx.db.player_teleport_event().insert(PlayerTeleportEvent { actor_id });
    Ok(())
}
