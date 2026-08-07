use bitcraft_macro::feature_gate;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::messages::action_request::PlayerTerraformSetFinalTargetRequest;
use crate::{
    game::{coordinates::ChunkCoordinates, game_state::game_state_filters},
    messages::{components::*, static_data::*},
    unwrap_or_err,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn terraform_set_final_target(ctx: &ReducerContext, request: PlayerTerraformSetFinalTargetRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::Terraform.get_layer(ctx),
        reduce(ctx, &mut cache, actor_id, &request),
    )
}

fn reduce(
    ctx: &ReducerContext,
    cache: &mut TerrainChunkCache,
    actor_id: u64,
    request: &PlayerTerraformSetFinalTargetRequest,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let terrain_coordinates = request.coordinates;
    let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinates::from_terrain_coordinates(terrain_coordinates.into()));
    // The terrain coordinates in the request are already scaled (LargeHexTile).
    let terrain_chunk = unwrap_or_err!(cache.filter_by_chunk_index(ctx, chunk_index), "Invalid terrain chunk");

    let offset_coordinates = terrain_coordinates;
    let terrain_cell = unwrap_or_err!(terrain_chunk.get_entity(&offset_coordinates), "Invalid terrain chunk");
    if terrain_cell.water_level > request.final_height_target {
        return Err("Can't dig below water level".into());
    }

    if request.final_height_target < 1 {
        return Err("Can't dig that low".into());
    }

    for direction in HexDirection::FLAT {
        let neighbor_coordinates = LargeHexTile::from(terrain_coordinates).neighbor(direction);
        let neighbor = unwrap_or_err!(
            cache.get_terrain_cell(ctx, &neighbor_coordinates),
            "Can't dig so close to the world edge"
        );
        if neighbor.is_submerged() && neighbor.water_level > request.final_height_target {
            return Err("Can't dig next to water".into());
        }
    }

    let player_coordinates = game_state_filters::coordinates_any(ctx, actor_id);

    if player_coordinates.parent_large_tile() != terrain_coordinates.into() {
        return Err("Invalid coordinates".into());
    }

    let building_entity_id = unwrap_or_err!(
        game_state_filters::building_id_at_coordinates(ctx, &game_state_filters::coordinates_any(ctx, actor_id)),
        "Player has to be in a building"
    );
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id),
        "Invalid terraforming building"
    );
    let building_description = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building.building_description_id),
        "Invalid building description"
    );

    if !building_description.has_category(ctx, BuildingCategory::TerrraformingBase) {
        return Err("Building isn't a terraforming base".into());
    }

    let terraform_progress_state = unwrap_or_err!(
        ctx.db.terraform_progress_state().entity_id().find(&building_entity_id),
        "terraform progress action not found"
    );

    let target_height_difference = i16::abs(request.final_height_target - terrain_cell.elevation);

    //cannot change more than 5
    if target_height_difference > 5 {
        return Err("Target elevation cannot be more than five from the current elevation".into());
    }

    //If terraform_set_final_target is called we should check if TerraformProgressState has already previously been set by another player.
    // To check this, we can check if final_height_target is different from the terrain elevation.
    //The progress field is zero: allow the final_height_target to be updated.
    // Note that the target can be updated if progress has been made, i.e. next_height_target is not the same as terrain elevation.
    // The actual progress field, i.e. the actions done to make one step change needs to be zero.
    if terraform_progress_state.progress > 0 {
        let request_direction = (request.final_height_target - terrain_cell.elevation).signum();
        let current_direction = (terraform_progress_state.final_height_target - terrain_cell.elevation).signum();

        //If the new final_height_target is in the opposite direction as the target in TerraformProgressState disallow the change with an error: “There is an opposing terraform action in progress”
        if current_direction != 0 && request_direction != current_direction {
            return Err("There is an opposing terraform action in progress".into());
        }
    }

    let mut terraform_progress_state = terraform_progress_state.clone();

    //outside of claims anyone can set the final_height_target and make terraform actions
    terraform_progress_state.validate_permission_set_final_elevation(ctx, actor_id, building.claim_entity_id)?;

    terraform_progress_state.final_height_target = request.final_height_target;

    let dir = if request.final_height_target > terrain_cell.elevation {
        1
    } else {
        -1
    };
    terraform_progress_state.next_height_target = terrain_cell.elevation + dir;
    ctx.db.terraform_progress_state().entity_id().update(terraform_progress_state);

    Ok(())
}
