use spacetimedb::{log, ReducerContext, Table};

use crate::game::coordinates::*;
use crate::game::reducer_helpers::deployable_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::*;
use crate::messages::static_data::*;

/// The terrain cell raised by a platform centered on `center`.
fn platform_cell(center: SmallHexTile) -> LargeHexTile {
    center.parent_large_tile()
}

/// Small tiles whose position lies on the platform cell: the 7 inner tiles and the cell's 6 corners.
fn tiles_on_cell(cell: LargeHexTile) -> impl Iterator<Item = SmallHexTile> {
    SmallHexTile::coordinates_in_radius_with_center_iter(cell.center_small_tile(), 2).filter(move |t| is_on_cell(*t, cell))
}

/// Whether a small tile lies on a platform cell (inner tiles, or one of the cell's corners).
fn is_on_cell(tile: SmallHexTile, cell: LargeHexTile) -> bool {
    tile.parent_large_tile() == cell || (tile.is_corner() && tile.get_terrain_coordinates().contains(&cell))
}

/// Players can only have a limited number of platforms of each type (`FloatingPlatformDesc::max_per_player`, 0 = unlimited).
pub fn validate_platform_limit(ctx: &ReducerContext, building_desc_id: i32, player_entity_id: u64) -> Result<(), String> {
    let Some(desc) = ctx.db.floating_platform_desc().building_id().find(&building_desc_id) else {
        return Ok(());
    };
    if desc.max_per_player <= 0 {
        return Ok(());
    }
    let built = ctx
        .db
        .building_state()
        .constructed_by_player_and_description()
        .filter((player_entity_id, building_desc_id))
        .count();
    if built >= desc.max_per_player as usize {
        let name = ctx
            .db
            .building_desc()
            .id()
            .find(&building_desc_id)
            .map_or("platforms".to_string(), |b| format!("{{0}}s|~{}", b.name));
        return Err(format!("You can't have more than {{0}} {{1}} placed at once.|~{}|~{}", desc.max_per_player, name));
    }
    Ok(())
}

/// The terrain cell of a completed floating platform building, or None if the building isn't a floating platform.
pub fn platform_cell_of(ctx: &ReducerContext, building_entity_id: u64) -> Option<LargeHexTile> {
    let building = ctx.db.building_state().entity_id().find(&building_entity_id)?;
    ctx.db
        .floating_platform_desc()
        .building_id()
        .find(&building.building_description_id)?;
    let location = ctx.db.location_state().entity_id().find(&building_entity_id)?;
    Some(platform_cell(location.coordinates()))
}

/// Raises the terrain cell under a newly created floating platform to its water level.
pub fn raise_terrain_under_platform(ctx: &ReducerContext, building_entity_id: u64, building_desc_id: i32) {
    if ctx.db.floating_platform_desc().building_id().find(&building_desc_id).is_none() {
        return;
    }
    let Some(location) = ctx.db.location_state().entity_id().find(&building_entity_id) else {
        log::error!("Floating platform {building_entity_id} has no location");
        return;
    };
    let cell = platform_cell(location.coordinates());
    let mut cache = TerrainChunkCache::empty();
    let Some(chunk) = cache.get_from_chunk_coordinates(ctx, cell.chunk_coordinates()) else {
        log::error!("Floating platform {building_entity_id}: no terrain chunk for {:?}", cell);
        return;
    };
    let index = chunk.get_index(cell);
    if chunk.water_levels[index] > chunk.elevations[index] {
        chunk.elevations[index] = chunk.water_levels[index];
        cache.consume_and_persist_all(ctx);
    }
}

/// Restores the seabed (the cell's original elevation) under a floating platform that is being removed.
pub fn restore_terrain_under_platform(ctx: &ReducerContext, building_desc_id: i32, location: SmallHexTile) {
    if ctx.db.floating_platform_desc().building_id().find(&building_desc_id).is_none() {
        return;
    }
    let cell = platform_cell(location);
    let mut cache = TerrainChunkCache::empty();
    let Some(chunk) = cache.get_from_chunk_coordinates(ctx, cell.chunk_coordinates()) else {
        log::error!("Floating platform at {:?}: no terrain chunk", cell);
        return;
    };
    let index = chunk.get_index(cell);
    chunk.elevations[index] = chunk.original_elevations[index];
    cache.consume_and_persist_all(ctx);
}

/// Dismounts riders in place after the platform's seabed is restored, before the building is deleted.
pub fn dismount_players_on_platform(ctx: &ReducerContext, building_entity_id: u64) {
    let Some(cell) = platform_cell_of(ctx, building_entity_id) else {
        return;
    };
    // Scan mountings rather than location chunks: a moving rider's origin can be in a different chunk.
    let riders = ctx
        .db
        .mounting_state()
        .iter()
        .filter_map(|mounting| {
            ctx.db.player_state().entity_id().find(mounting.entity_id)?;
            let mobile = ctx.db.mobile_entity_state().entity_id().find(mounting.entity_id)?;
            let position = mobile.interpolated_location(ctx);
            is_on_cell(position.parent_small_tile(), cell).then_some((mounting.entity_id, position))
        });

    for (player_entity_id, position) in riders {
        ctx.db.mobile_entity_state().entity_id().update(MobileEntityState::for_location(
            player_entity_id,
            position.into(),
            ctx.timestamp,
        ));
        deployable_helpers::dismount_deployable(ctx, player_entity_id, false);
        PlayerState::collect_stats(ctx, player_entity_id);
    }
}

/// Whether a non-player, non-deployable mobile entity (pet, mob...) is on, or moving onto, the platform's cell. Removing the platform
/// would strand it in the water, so deconstruction and decay refuse to. Players and deployables don't count: platforms can be removed
/// underneath them.
pub fn is_platform_occupied(ctx: &ReducerContext, building_entity_id: u64) -> bool {
    let Some(cell) = platform_cell_of(ctx, building_entity_id) else {
        return false;
    };
    // Mobile entities are indexed by chunk: only scan the chunk(s) the platform cell's tiles are in (almost always one).
    let mut chunk_indexes: Vec<u64> = tiles_on_cell(cell).map(|t| t.chunk_coordinates().chunk_index()).collect();
    chunk_indexes.sort_unstable();
    chunk_indexes.dedup();
    chunk_indexes.into_iter().any(|chunk_index| {
        ctx.db
            .mobile_entity_state()
            .chunk_index()
            .filter(chunk_index)
            // Also check the destination so something walking onto the platform counts too
            .filter(|mobile| is_on_cell(mobile.coordinates(), cell) || is_on_cell(mobile.destination(), cell))
            .any(|mobile| {
                ctx.db.player_state().entity_id().find(&mobile.entity_id).is_none()
                    && ctx.db.deployable_state_v2().entity_id().find(&mobile.entity_id).is_none()
            })
    })
}

/// A platform can only be removed when no blocking mobile entity is on it (players and deployables are allowed)
/// and, for platforms that support buildings, once nothing else is built on it.
pub fn validate_platform_removal(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    if platform_cell_of(ctx, building_entity_id).is_none() {
        return Ok(());
    }
    let occupied_by_mobile = is_platform_occupied(ctx, building_entity_id);
    if occupied_by_mobile {
        return Err("Can't remove the platform while something is on it.".into());
    }

    for platform_tile in ctx.db.footprint_tile_state().owner_entity_id().filter(building_entity_id) {
        let Some(location) = ctx.db.location_state().entity_id().find(&platform_tile.entity_id) else {
            continue;
        };
        let occupied = FootprintTileState::get_at_location(ctx, &location.coordinates())
            .any(|f| f.owner_entity_id != building_entity_id && f.footprint_type != FootprintType::Perimeter);
        if occupied {
            return Err("Remove everything built on this platform first.".into());
        }
    }
    Ok(())
}
