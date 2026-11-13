use crate::game::autogen::_delete_entity::delete_entity;
use crate::game::coordinates::*;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::{
    game::{discovery::Discovery, game_state},
    messages::components::PavedTileState,
    messages::{action_request::PlayerPavingPlaceTileRequest, components::ResourceState},
    unwrap_or_err,
};
use crate::{paved_tile_state, paving_tile_desc};
use spacetimedb::{ReducerContext, Table};

// Similar to paving_add_tile::reduce()
#[spacetimedb::reducer]
pub fn cheat_paving_add_tile(ctx: &ReducerContext, request: PlayerPavingPlaceTileRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatPavingAddTile) {
        return Err("Unauthorized.".into());
    }

    let coordinates: SmallHexTile = request.coordinates.into();
    let paving_type_id: i32 = request.tile_type_id;

    let target_coord = coordinates;

    let mut terrain_cache = TerrainChunkCache::empty();
    if let Some(terrain_target) = terrain_cache.get_terrain_cell(ctx, &target_coord.parent_large_tile()) {
        if terrain_target.is_submerged() {
            return Err("Can't pave under water".into());
        }
    } else {
        return Err("Invalid coordinates".into());
    }

    // for footprint in FootprintTileState::get_at_location(&target_coord) {
    //     if footprint.footprint_type != FootprintType::Perimeter
    //         && footprint.footprint_type != FootprintType::WalkableResource
    //     {
    //         return Err("Something is blocking the way".into());
    //     }
    // }

    if !game_state::game_state_filters::is_flat_corner(ctx, &mut terrain_cache, target_coord) {
        return Err("Can only pave flat terrain".into());
    }

    if let Some(existing_pave) = PavedTileState::get_at_location(ctx, &target_coord) {
        let existing_paving_entity_id = existing_pave.entity_id;
        delete_entity(ctx, existing_paving_entity_id);
    }

    let _tile_description = unwrap_or_err!(ctx.db.paving_tile_desc().id().find(paving_type_id), "Invalid paving tile type");

    // Create paved tile
    let entity_id = game_state::create_entity(ctx);

    // location
    let offset = target_coord.to_offset_coordinates();
    game_state::insert_location(ctx, entity_id, offset);

    // tile entity
    let paved_tile = PavedTileState {
        entity_id,
        tile_type_id: paving_type_id,
        related_entity_id: 0,
    };

    if ctx.db.paved_tile_state().try_insert(paved_tile).is_err() {
        return Err("Failed to insert pavement".into());
    }

    // Despawn resources under paving
    if let Some(deposit) = ResourceState::get_at_location(ctx, &target_coord) {
        deposit.despawn_self(ctx);
    }

    let actor_id = game_state::actor_id(ctx, false)?;
    let mut discovery = Discovery::new(actor_id);
    discovery.acquire_paving(ctx, paving_type_id);
    discovery.commit(ctx);

    Ok(())
}
