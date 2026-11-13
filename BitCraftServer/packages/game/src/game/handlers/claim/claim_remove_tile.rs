use bitcraft_macro::feature_gate;
use std::collections::HashSet;

use spacetimedb::ReducerContext;

use crate::{
    building_claim_desc, claim_state, claim_tile_state,
    game::{
        claim_helper,
        coordinates::SmallHexTile,
        dimensions,
        game_state::{self, game_state_filters},
    },
    location_state,
    messages::{action_request::PlayerClaimRemoveTileRequest, components::claim_local_state},
    unwrap_or_err, PlayerTimestampState,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_remove_tile(ctx: &ReducerContext, request: PlayerClaimRemoveTileRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let coord = SmallHexTile::from(request.coordinates);
    reduce(ctx, actor_id, coord)
}

fn reduce(ctx: &ReducerContext, actor_id: u64, tile: SmallHexTile) -> Result<(), String> {
    if tile.dimension != dimensions::OVERWORLD {
        return Err("Cannot unclaim tiles inside an interior.".into());
    }

    // Make sure the tile is claimed
    if let Some(claimed_tile) = claim_helper::get_claim_on_tile(ctx, tile) {
        let claim_entity_id = claimed_tile.claim_id;
        let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "Claim does not exist");
        let mut claim_local = claim.local_state(ctx);

        // Make sure player has permission
        if !claim.has_co_owner_permissions(ctx, actor_id) {
            return Err("You don't have permission to modify claimed territory.".into());
        }

        let claim_building = unwrap_or_err!(claim_helper::get_claim_building(ctx, claim_entity_id), "Claim has no building");
        let claim_radius = ctx
            .db
            .building_claim_desc()
            .building_id()
            .find(claim_building.building_description_id)
            .unwrap()
            .radius;
        let claim_location = game_state_filters::coordinates_any(ctx, claim_building.entity_id);

        if claim_location.distance_to(tile) <= claim_radius {
            return Err("Cannot clear initial claimed land.".into());
        }

        if game_state_filters::building_at_coordinates(ctx, &tile).is_some() {
            return Err("Cannot unclaim area under a building".into());
        }

        if game_state_filters::project_site_at_coordinates(ctx, &tile).is_some() {
            return Err("Cannot unclaim area under a project site".into());
        }

        // Make sure removing this claim won't orphan an area
        let mut claim_tiles: HashSet<SmallHexTile> = ctx
            .db
            .claim_tile_state()
            .claim_id()
            .filter(claim_entity_id)
            .map(|c| ctx.db.location_state().entity_id().find(c.entity_id).unwrap().coordinates())
            .collect();
        claim_tiles.remove(&tile);
        let mut open_list: Vec<SmallHexTile> = Vec::with_capacity(claim_tiles.len());
        open_list.push(claim_location);
        let mut closed_list: HashSet<SmallHexTile> = HashSet::with_capacity(claim_tiles.len());

        // Flood-fill starting with each neighbor
        while let Some(cur) = open_list.pop() {
            for n in cur.neighbors_direct() {
                if !claim_tiles.contains(&n) || open_list.contains(&n) || closed_list.contains(&n) {
                    continue;
                }
                open_list.push(n);
            }
            closed_list.insert(cur);
        }

        if closed_list.len() < claim_tiles.len() {
            return Err("Cannot orphan claimed area".into());
        }

        let mut num_adjacent_tiles = 0;
        for direct_neighbor in tile.neighbors_direct() {
            if claim_tiles.contains(&direct_neighbor) {
                num_adjacent_tiles += 1;
            }
        }

        claim_local.num_tiles -= 1;
        claim_local.num_tile_neighbors -= num_adjacent_tiles as u32 * 2;
        ctx.db.claim_local_state().entity_id().update(claim_local);

        claim_helper::unclaim_tile(ctx, claimed_tile.entity_id, tile);
        return Ok(());
    }
    // This is not even a claimed tile, how did that happen?
    Err("Not claimed.".into())
}
