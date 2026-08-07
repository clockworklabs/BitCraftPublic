use crate::game::coordinates::*;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::{game::coordinates::ChunkCoordinates, messages::components::TerrainChunkState, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_terraform(ctx: &ReducerContext, x: i32, z: i32, dimension: u32, delta: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender, CheatType::CheatTerraform) {
        return Err("Unauthorized.".into());
    }

    let terrain_coordinates = OffsetCoordinatesLarge { x, z, dimension };
    let offset_coordinates = terrain_coordinates;

    let mut terrain_cache = TerrainChunkCache::empty();
    // The terrain coordinates in the request are already scaled (LargeHexTile).
    let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinates::from_terrain_coordinates(terrain_coordinates.into()));
    let mut terrain_cell = unwrap_or_err!(
        unwrap_or_err!(terrain_cache.filter_by_chunk_index(ctx, chunk_index), "Invalid terrain chunk").get_entity(&offset_coordinates),
        "Invalid terrain chunk"
    );

    let delta = delta as i16;
    if delta < 0 {
        for direction in HexDirection::FLAT {
            let neighbor_coordinates = LargeHexTile::from(terrain_coordinates).neighbor(direction);
            let _neighbor = unwrap_or_err!(
                terrain_cache.get_terrain_cell(ctx, &neighbor_coordinates),
                "Can't dig so close to the world edge"
            );
        }
    }

    let target = terrain_cell.elevation + delta;

    terrain_cell.elevation = target;
    terrain_cache
        .filter_by_chunk_index(ctx, chunk_index)
        .expect("we already verified that the state exists")
        .set_entity(offset_coordinates, terrain_cell);
    terrain_cache.consume_and_persist(ctx, chunk_index);

    Ok(())
}
