use std::collections::HashMap;

use game::dimensions;
use table_caches::location_state_cache::LocationStateCache;

use crate::*;

pub struct ClaimTileStateCache<'a> {
    location_cache: &'a mut LocationStateCache,
    claim_by_coord: HashMap<i64, Option<u64>>,
}

impl<'a> ClaimTileStateCache<'a> {
    pub fn new(location_cache: &'a mut LocationStateCache) -> Self {
        Self {
            location_cache,
            claim_by_coord: HashMap::with_capacity(128),
        }
    }

    pub fn location_cache(&mut self) -> &mut LocationStateCache {
        self.location_cache
    }

    pub fn get_claim_on_tile(&mut self, ctx: &ReducerContext, coord: SmallHexTile) -> Option<u64> {
        *self.claim_by_coord.entry(coord.hashcode()).or_insert_with(|| {
            self.location_cache
                .select_all(ctx, &coord)
                .iter()
                .filter_map(|c| ctx.db.claim_tile_state().entity_id().find(c))
                .map(|c| c.claim_id)
                .next()
        })
    }

    pub fn add_claim_on_tile(&mut self, entity_id: u64, claim_entity_id: u64, coord: &SmallHexTile) {
        self.location_cache.add_location_entity(entity_id, coord);
        let entry = self.claim_by_coord.entry(coord.hashcode()).or_insert(Some(claim_entity_id));
        if entry.is_none() {
            entry.replace(claim_entity_id);
        }
    }

    pub fn any_claim_in_radius(
        &mut self,
        ctx: &ReducerContext,
        coord: SmallHexTile,
        radius: i32,
        neutral_radius: i32,
        ignore_neutral_claims: bool,
        claim_to_exclude: u64,
    ) -> Option<u64> {
        if coord.dimension != dimensions::OVERWORLD {
            panic!("claims_in_radius should only be called in overworld");
        }

        let search_radius = if ignore_neutral_claims { radius } else { radius.max(neutral_radius) };
        let mut neutral_by_claim = HashMap::new();
        SmallHexTile::coordinates_in_radius_with_center_iter(coord, search_radius).find_map(|tile| {
            let claim_id = self.get_claim_on_tile(ctx, tile)?;
            if claim_id == claim_to_exclude {
                return None;
            }
            let neutral = *neutral_by_claim.entry(claim_id).or_insert_with(|| {
                ctx.db.claim_state().entity_id().find(claim_id).unwrap().neutral
            });
            if neutral && ignore_neutral_claims {
                return None;
            }
            let required_distance = if neutral { neutral_radius } else { radius };
            (tile.distance_to(coord) <= required_distance).then_some(claim_id)
        })
    }

    pub fn get_num_adjacent_tiles(&mut self, ctx: &ReducerContext, coord: SmallHexTile) -> usize {
        if coord.dimension != dimensions::OVERWORLD {
            panic!("get_num_adjacent_tiles should only be called in overworld");
        }

        SmallHexTile::coordinates_in_radius(coord, 1)
            .iter()
            .filter_map(|t| self.get_claim_on_tile(ctx, *t))
            .count()
    }

    /*
    //Due to some lifetime bs this can only be called once, so use the two methods above instead
    pub fn claims_in_radius_iter(&'a mut self, coord: SmallHexTile, radius: i32) -> impl Iterator<Item = u64> + '_ {
        if coord.dimension != dimensions::OVERWORLD {
            panic!("claims_in_radius should only be called in overworld");
        }

        return SmallHexTile::coordinates_in_radius_with_center_iter(coord, radius).filter_map(|t| self.get_claim_on_tile(t));
    } */
}
