use crate::floating_platform_desc;
use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    building_desc,
    game::{
        coordinates::*,
        dimensions,
        game_state::{self, game_state_filters},
        handlers::player::player_use_elevator::player_use_elevator_timer,
        permission_helper,
        reducer_helpers::building_helpers::move_building_unsafe,
        terrain_chunk::TerrainChunkCache,
    },
    messages::{action_request::PlayerBuildingMoveRequest, components::*},
    unwrap_or_err, BuildingCategory,
};

#[shared_table_reducer]
#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn building_move(ctx: &ReducerContext, request: PlayerBuildingMoveRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building_entity_id = request.building_entity_id;
    let building_state = unwrap_or_err!(ctx.db.building_state().entity_id().find(&building_entity_id), "Invalid building").clone();

    let coordinates = request.new_coordinates;
    let player_housing = PlayerHousingState::from_dimension(ctx, coordinates.dimension);
    let original_claim_id = building_state.claim_entity_id;

    if player_housing.is_none() && original_claim_id == 0 {
        return Err("You can only move a building that is part of a claim.".into());
    }

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    // Housing only uses the new PermissionState
    if player_housing.is_none() {
        if !permission_helper::can_interact_with_building(ctx, &building_state, actor_id, ClaimPermission::Build) {
            return Err("You don't have permission to interact with this building".into());
        }
    }

    let previous_coordinates = game_state_filters::coordinates_any(ctx, building_entity_id);

    if coordinates.dimension != previous_coordinates.dimension {
        return Err("You can't move a building through a portal".into());
    }

    if ctx
        .db
        .player_use_elevator_timer()
        .origin_platform_entity_id()
        .find(&building_entity_id)
        .is_some()
        || ctx
            .db
            .player_use_elevator_timer()
            .destination_platform_entity_id()
            .find(&building_entity_id)
            .is_some()
    {
        return Err("You can't move an elevator that's in use".into());
    }

    let building = ctx.db.building_desc().id().find(&building_state.building_description_id).unwrap();

    if building.has_category(ctx, BuildingCategory::TerrraformingBase) {
        return Err("Terraforming digsites can't be moved.".into());
    }

    if ctx.db.floating_platform_desc().building_id().find(&building.id).is_some() {
        return Err("Floating platforms can't be moved.".into());
    }

    if building.has_category(ctx, BuildingCategory::ClaimTotem) {
        return Err("Claim totems can't be moved.".into());
    }

    if building.has_category(ctx, BuildingCategory::Portal) && coordinates.dimension != dimensions::OVERWORLD {
        return Err("Interior portals cannot be moved".into());
    }

    let mut terrain_cache = TerrainChunkCache::empty();

    // Validate that all new module positions are valid and within the same claim
    match ProjectSiteState::validate_building_placement(
        ctx,
        &mut terrain_cache,
        coordinates.into(),
        HexDirection::from(request.facing_direction),
        actor_id,
        &building,
        false,
        original_claim_id,
        Some(building_entity_id),
    ) {
        Err(str) => return Err(str.into()),
        _ => (),
    };

    move_building_unsafe(ctx, building_entity_id, coordinates, request.facing_direction)
}
