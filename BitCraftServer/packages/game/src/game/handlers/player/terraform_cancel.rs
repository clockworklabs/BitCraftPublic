use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self, game_state_filters},
    messages::{action_request::PlayerTerraformCancelRequest, components::*, static_data::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn terraform_cancel(ctx: &ReducerContext, request: PlayerTerraformCancelRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let terrain_coordinates = request.coordinates;

    let player_coordinates = game_state_filters::coordinates_any(ctx, actor_id);

    if player_coordinates.parent_large_tile() != terrain_coordinates.into() {
        return Err("Invalid coordinates".into());
    }

    let building_entity_id = unwrap_or_err!(
        game_state_filters::building_id_at_coordinates(ctx, &player_coordinates),
        "Player has to be in a building"
    );
    let building = ctx.db.building_state().entity_id().find(building_entity_id).unwrap();
    let building_description = unwrap_or_err!(
        ctx.db.building_desc().id().find(building.building_description_id),
        "Invalid building description"
    );

    if !building_description.has_category(ctx, BuildingCategory::TerrraformingBase) {
        return Err("Building isn't a terraforming base".into());
    }

    let mut terraform_progress_state = unwrap_or_err!(
        ctx.db.terraform_progress_state().entity_id().find(building_entity_id),
        "terraform progress state not found"
    );

    terraform_progress_state.validate_permission_terraform(ctx, actor_id, 0)?;

    terraform_progress_state.reset();

    ctx.db.terraform_progress_state().entity_id().update(terraform_progress_state);

    Ok(())
}
