use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        claim_helper,
        coordinates::SmallHexTile,
        dimensions,
        game_state::{self, game_state_filters},
    },
    messages::{action_request::PlayerClaimAddTileRequest, components::*},
    parameters_desc,
    table_caches::{claim_tile_state_cache::ClaimTileStateCache, location_state_cache::LocationStateCache},
    unwrap_or_err, ChunkCoordinates,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_add_tile(ctx: &ReducerContext, request: PlayerClaimAddTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let coord = SmallHexTile::from(request.coordinates);
    reduce(ctx, actor_id, request.claim_entity_id, coord)
}

fn reduce(ctx: &ReducerContext, actor_id: u64, claim_entity_id: u64, t: SmallHexTile) -> Result<(), String> {
    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(&claim_entity_id),
        "Claim does not exist"
    );
    let mut claim_local = claim.local_state(ctx);

    // Make sure player has permission
    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("You don't have permission to modify claimed territory.".into());
    }

    if t.dimension != dimensions::OVERWORLD {
        return Err("Cannot claim times inside an interior.".into());
    }

    let mut location_cache = LocationStateCache::new();
    let mut claim_cache = ClaimTileStateCache::new(&mut location_cache);

    // Make sure the tile is not already claimed
    if claim_cache.get_claim_on_tile(ctx, t).is_some() {
        return Err("Already claimed.".into());
    }

    // Make sure the tile is adjacent to a claimed tile
    let num_adjacent_tiles = claim_cache.get_num_adjacent_tiles(ctx, t);
    if num_adjacent_tiles == 0 {
        return Err("Must select a tile adjacent to the claim".into());
    }

    let parent_large_tile = t.parent_large_tile();
    let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinates::from_terrain_coordinates(parent_large_tile.into()));
    let terrain_chunk = unwrap_or_err!(
        ctx.db.terrain_chunk_state().chunk_index().find(&chunk_index),
        "Invalid terrain chunk"
    );
    let terrain_cell = unwrap_or_err!(terrain_chunk.get_entity(&parent_large_tile.into()), "Invalid terrain chunk");
    if terrain_cell.biome_percentage(Biome::SafeMeadows) > 0f32 {
        return Err("Cannot claim land in this biome".into());
    }

    // Make sure the location is not within range of a different claim
    let min_distance_between_claims = ctx.db.parameters_desc().version().find(0).unwrap().min_distance_between_claims;

    let is_close_to_other_claim = claim_cache
        .any_claim_in_radius_except(ctx, t, min_distance_between_claims, claim_entity_id)
        .is_some();
    if is_close_to_other_claim {
        return Err("Cannot claim land too close to another claimed area".into());
    }

    let max_tiles = ctx.db.claim_tech_state().entity_id().find(&claim_entity_id).unwrap().max_tiles(ctx);
    if claim_local.num_tiles >= max_tiles {
        return Err("Claim can't be extended further, you need to develop your tech.".into());
    }

    let claim_center = game_state_filters::coordinates(ctx, claim.owner_building_entity_id);
    if !SmallHexTile::simple_raycast(&t, &claim_center, |cur| {
        return match claim_cache.get_claim_on_tile(ctx, cur) {
            Some(ct) => ct == claim_entity_id,
            None => true,
        };
    }) {
        return Err("Cannot claim tiles that would partially surround another claim".into());
    }

    claim_local.num_tiles += 1;
    claim_local.num_tile_neighbors += num_adjacent_tiles as u32 * 2;
    ctx.db.claim_local_state().entity_id().update(claim_local);

    claim_helper::claim_tile(ctx, claim_entity_id, t, true, &mut claim_cache);
    Ok(())
}
