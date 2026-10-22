use std::cell::Cell;

use spacetimedb::{ReducerContext, Table};

use crate::{
    messages::{components::*, static_data::*},
    table_caches::{claim_tile_state_cache::ClaimTileStateCache, location_state_cache::LocationStateCache},
    unwrap_or_err, unwrap_or_return,
};

use super::{
    dimensions,
    entities::building_state::BuildingState,
    game_state::{self, game_state_filters},
    terrain_chunk::TerrainChunkCache,
};
use crate::game::coordinates::*;

pub fn get_claim_building(ctx: &ReducerContext, claim_entity_id: u64) -> Option<BuildingState> {
    if let Some(claim) = ctx.db.claim_state().entity_id().find(&claim_entity_id) {
        return ctx.db.building_state().entity_id().find(&claim.owner_building_entity_id);
    };
    None
}

// Returns the claim_entity_id of the tiles under the building if every walkable and hitbox footprints are covered,
// 0 otherwise.
pub fn get_claim_under_building(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    building_id: i32,
    direction: i32,
    claim_cache: &mut ClaimTileStateCache,
) -> u64 {
    let building_desc = ctx.db.building_desc().id().find(&building_id).unwrap();
    let footprint = building_desc.get_footprint(&coordinates, direction);
    get_claim_under_footprint_cached(ctx, &footprint, claim_cache)
}

// Returns the claim_entity_id of the tiles under the building if every walkable and hitbox footprints are covered,
// 0 otherwise.
pub fn get_claim_under_resource(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    resource_id: i32,
    direction: i32,
    claim_cache: &mut ClaimTileStateCache,
) -> u64 {
    let resource_desc = ctx.db.resource_desc().id().find(&resource_id).unwrap();
    let footprint = resource_desc.get_footprint(&coordinates, direction);
    get_claim_under_footprint_cached(ctx, &footprint, claim_cache)
}

// Returns the claim_entity_id of the tiles under the footprint if every walkable and hitbox footprints are covered,
// 0 otherwise.
pub fn get_claim_under_footprint_cached(
    ctx: &ReducerContext,
    footprint: &Vec<(SmallHexTile, FootprintType)>,
    claim_cache: &mut ClaimTileStateCache,
) -> u64 {
    let dimension = footprint[0].0.dimension;
    if dimension != dimensions::OVERWORLD {
        return match DimensionNetworkState::get(ctx, dimension) {
            Some(dn) => dn.claim_entity_id,
            None => 0,
        };
    }

    let mut claim_entity_id = 0;
    for (coord, footprint_type) in footprint {
        if *footprint_type == FootprintType::Walkable
            || *footprint_type == FootprintType::Hitbox
            || *footprint_type == FootprintType::Platform
        {
            if let Some(claim_id) = claim_cache.get_claim_on_tile(ctx, *coord) {
                if claim_entity_id != 0 && claim_entity_id != claim_id {
                    // split between 2 different claims => not claimable
                    return 0;
                }
                claim_entity_id = claim_id;
            } else {
                return 0;
            }
        }
    }
    claim_entity_id
}

// Returns the claim_entity_id of the tiles under the footprint if every walkable and hitbox footprints are covered,
// 0 otherwise.
pub fn get_claim_under_footprint(ctx: &ReducerContext, footprint: &Vec<(SmallHexTile, FootprintType)>) -> u64 {
    let dimension = footprint[0].0.dimension;
    if dimension != dimensions::OVERWORLD {
        return match DimensionNetworkState::get(ctx, dimension) {
            Some(dn) => dn.claim_entity_id,
            None => 0,
        };
    }

    let mut claim_entity_id = 0;
    for (coord, footprint_type) in footprint {
        if *footprint_type == FootprintType::Walkable
            || *footprint_type == FootprintType::Hitbox
            || *footprint_type == FootprintType::Platform
        {
            if let Some(claim) = get_claim_on_tile(ctx, *coord) {
                if claim_entity_id != 0 && claim_entity_id != claim.claim_id {
                    // split between 2 different claims => not claimable
                    return 0;
                }
                claim_entity_id = claim.claim_id;
            } else {
                return 0;
            }
        }
    }
    claim_entity_id
}

pub fn get_partial_claims_under_building(ctx: &ReducerContext, coordinates: SmallHexTile, building_id: i32, direction: i32) -> Vec<u64> {
    let building_desc = ctx.db.building_desc().id().find(&building_id).unwrap();
    let footprint = building_desc.get_footprint(&coordinates, direction);
    get_partial_claims_under_footprint(ctx, &footprint)
}

pub fn get_partial_claims_under_footprint(ctx: &ReducerContext, footprint: &Vec<(SmallHexTile, FootprintType)>) -> Vec<u64> {
    let mut claims = Vec::new();
    let mut first_claim = 0; //If there's only one claim under this building, it will be stored in this variable to reduce memory overhead. If there's more, then there's no way to avoid that overhead
    for (coord, footprint_type) in footprint {
        if *footprint_type == FootprintType::Walkable
            || *footprint_type == FootprintType::Hitbox
            || *footprint_type == FootprintType::Platform
        {
            if let Some(claim) = get_claim_on_tile(ctx, *coord) {
                if first_claim == 0 {
                    first_claim = claim.claim_id;
                    claims.push(first_claim);
                } else if claim.claim_id != first_claim {
                    claims.push(claim.claim_id);
                }
            }
        }
    }

    if claims.len() > 1 {
        claims.sort();
        claims.dedup();
    }
    claims
}

pub fn claim_tile(
    ctx: &ReducerContext,
    claim_entity_id: u64,
    coord: SmallHexTile,
    claim_buildings: bool,
    mut claim_cache: &mut ClaimTileStateCache,
) {
    let entity_id = game_state::create_entity(ctx);
    let offset = coord.to_offset_coordinates();

    game_state::insert_location(ctx, entity_id, offset);
    if ctx
        .db
        .claim_tile_state()
        .try_insert(ClaimTileState {
            entity_id,
            claim_id: claim_entity_id,
        })
        .is_err()
    {
        panic!("Failed to insert ClaimTileState (entity_id: {entity_id}, claim_id: {claim_entity_id})");
    }

    claim_cache.add_claim_on_tile(entity_id, claim_entity_id, &coord);

    if claim_buildings {
        claim_tile_buildings(ctx, claim_entity_id, coord, &mut claim_cache);
    }

    for mut deployable in game_state_filters::deployables_at_coordinates(ctx, coord) {
        deployable.claim_entity_id = claim_entity_id;

        ctx.db.deployable_state_v2().entity_id().update(deployable);
    }
}

fn claim_tile_buildings(ctx: &ReducerContext, claim_entity_id: u64, coord: SmallHexTile, claim_cache: &mut ClaimTileStateCache) {
    // Check if a building or project site is on the added tile
    let footprint = claim_cache
        .location_cache()
        .select_all(ctx, &coord)
        .iter()
        .filter_map(|c| ctx.db.footprint_tile_state().entity_id().find(c))
        .filter(|f| {
            f.footprint_type == FootprintType::Hitbox
                || f.footprint_type == FootprintType::Walkable
                || f.footprint_type == FootprintType::Platform
        })
        .map(|f| f.owner_entity_id)
        .next();
    if let Some(building_entity_id) = footprint {
        let building_location = game_state_filters::coordinates_any(ctx, building_entity_id);
        if let Some(building) = ctx.db.building_state().entity_id().find(&building_entity_id) {
            if get_claim_under_building(
                ctx,
                building_location,
                building.building_description_id,
                building.direction_index,
                claim_cache,
            ) == claim_entity_id
            {
                BuildingState::claim(ctx, building_entity_id, claim_entity_id);
            }
        } else if let Some(mut project_site) = ctx.db.project_site_state().entity_id().find(&building_entity_id) {
            if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(&project_site.construction_recipe_id) {
                let building_id = recipe.building_description_id;
                if get_claim_under_building(ctx, building_location, building_id, project_site.direction, claim_cache) == claim_entity_id {
                    project_site.owner_id = claim_entity_id;
                    ctx.db.project_site_state().entity_id().update(project_site);
                }
            } else if let Some(recipe) = ctx
                .db
                .resource_placement_recipe_desc()
                .id()
                .find(&project_site.resource_placement_recipe_id)
            {
                let resource_id = recipe.resource_description_id;
                if get_claim_under_resource(ctx, building_location, resource_id, project_site.direction, claim_cache) == claim_entity_id {
                    project_site.owner_id = claim_entity_id;
                    ctx.db.project_site_state().entity_id().update(project_site);
                }
            }
        }
    }
}

pub fn unclaim_tile(ctx: &ReducerContext, claim_tile_entity_id: u64, coord: SmallHexTile) {
    ctx.db.claim_tile_state().entity_id().delete(&claim_tile_entity_id);
    ctx.db.location_state().entity_id().delete(&claim_tile_entity_id);

    // Check if a building is on the removed tile, if so it's no longer fully covered by claim tiles therefore no longer claimed
    if let Some(building) = game_state_filters::building_at_coordinates(ctx, &coord) {
        unclaim_building(ctx, building, false);
    }

    for deployable in game_state_filters::deployables_at_coordinates(ctx, coord) {
        unclaim_deployable(ctx, deployable);
    }
}

pub fn delete_all_claim_tiles(ctx: &ReducerContext, claim_entity_id: u64) {
    for building in ctx.db.building_state().claim_entity_id().filter(claim_entity_id) {
        unclaim_building(ctx, building, false);
    }

    for deployable in ctx.db.deployable_state_v2().claim_entity_id().filter(claim_entity_id) {
        unclaim_deployable(ctx, deployable);
    }

    for tile in ctx.db.claim_tile_state().claim_id().filter(claim_entity_id) {
        let entity_id = tile.entity_id;
        ctx.db.location_state().entity_id().delete(&entity_id);
        ctx.db.claim_tile_state().entity_id().delete(&entity_id);
    }
}

fn unclaim_building(ctx: &ReducerContext, building: BuildingState, from_destructor: bool) {
    let building_entity_id = building.entity_id;
    building.unclaim_self(ctx, false, from_destructor);

    if let Some(project_site) = ctx.db.project_site_state().entity_id().find(building_entity_id) {
        if project_site.owner_id != 0 {
            let mut project_site = project_site;
            project_site.owner_id = 0;
            ctx.db.project_site_state().entity_id().update(project_site);
        }
    }
}

fn unclaim_deployable(ctx: &ReducerContext, mut deployable: DeployableStateV2) {
    deployable.claim_entity_id = 0;
    ctx.db.deployable_state_v2().entity_id().update(deployable);
}

pub fn claim_area_around_totem(ctx: &ReducerContext, claim_entity_id: u64, radius: i32, ignore_neutral_claims: bool) -> bool {
    if let Some(building) = get_claim_building(ctx, claim_entity_id) {
        let claim_building_coordinates = game_state_filters::coordinates_any(ctx, building.entity_id);

        let mut location_cache = LocationStateCache::new();
        let mut claim_cache = ClaimTileStateCache::new(&mut location_cache);

        if DONT_CHECK_AREA_AROUND_CLAIM_COUNTER.get() <= 0 {
            let min_distance_between_claims = ctx.db.parameters_desc().version().find(&0).unwrap().min_distance_between_claims;
            if claim_cache
                .any_claim_in_radius(
                    ctx,
                    claim_building_coordinates,
                    min_distance_between_claims + radius,
                    ignore_neutral_claims,
                )
                .is_some()
            {
                return false;
            }
        }

        //Claim tiles
        claim_tile(ctx, claim_entity_id, claim_building_coordinates, false, &mut claim_cache);
        let claim_coordinates = SmallHexTile::coordinates_in_radius(claim_building_coordinates, radius);
        for coord in &claim_coordinates {
            claim_tile(ctx, claim_entity_id, *coord, false, &mut claim_cache);
        }

        //Claim buildings under tiles
        claim_tile_buildings(ctx, claim_entity_id, claim_building_coordinates, &mut claim_cache);
        for coord in claim_coordinates {
            claim_tile_buildings(ctx, claim_entity_id, coord, &mut claim_cache);
        }

        return true;
    }
    false
}

const MIN_CLAIM_TOTEM_SMALL_TILE_DISTANCE: i32 = 100;

pub fn can_place_claim_totem(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    claim_info: &BuildingClaimDesc,
    terrain_cache: &mut TerrainChunkCache,
) -> Result<(), String> {
    if game_state_filters::any_claim_totems_in_radius(ctx, coordinates, MIN_CLAIM_TOTEM_SMALL_TILE_DISTANCE) {
        return Err("Cannot start a settlement too close to another player’s settlement".into());
    }

    if game_state_filters::any_claims_in_radius(
        ctx,
        coordinates,
        ctx.db.parameters_desc().version().find(&0).unwrap().min_distance_between_claims + claim_info.radius,
    ) {
        return Err("Claimed area would be too close to another claim".into());
    }

    // Don't allow claims that overlap prohibited biomes
    let prohibited_biomes: Vec<Biome> = ctx
        .db
        .biome_desc()
        .disallow_player_build()
        .filter(true)
        .map(|b| Biome::to_enum(b.biome_type))
        .collect();
    for tile in SmallHexTile::coordinates_in_radius_with_center_iter(coordinates, claim_info.radius) {
        for biome in &prohibited_biomes {
            let terrain_target = unwrap_or_err!(
                terrain_cache.get_terrain_cell(ctx, &tile.parent_large_tile()),
                "Claim outside world bounds"
            );

            if terrain_target.biome_percentage(*biome) > 0f32 {
                return Err("You can't claim land in this area".into());
            }
        }
    }

    Ok(())
}

pub fn get_claim_on_tile(ctx: &ReducerContext, coordinates: SmallHexTile) -> Option<ClaimTileState> {
    if coordinates.dimension != dimensions::OVERWORLD {
        if let Some(dimension_description_network) = DimensionNetworkState::get(ctx, coordinates.dimension) {
            if dimension_description_network.claim_entity_id != 0 {
                // DAB Note: This isn't great. We probably should return the claim instead of the ClaimTile, but for now let's go with a temp ClaimTileState.
                return Some(ClaimTileState {
                    entity_id: 0,
                    claim_id: dimension_description_network.claim_entity_id,
                });
            }
        }
        return None;
    }

    return ClaimTileState::get_at_location(ctx, &coordinates);
}

pub fn mint_hex_coins(ctx: &ReducerContext, claim_entity_id: u64, gained_xp: u32) {
    let claim = unwrap_or_return!(ctx.db.claim_state().entity_id().find(&claim_entity_id), "Unknown claim");
    let claim_tech = unwrap_or_return!(ctx.db.claim_tech_state().entity_id().find(&claim_entity_id), "Unknown claim tech");
    let xp_to_mint_hex_coin = claim_tech.min_xp_to_mint_hex_coin(ctx);
    let mut claim_local = claim.local_state(ctx);

    if xp_to_mint_hex_coin == u32::MAX {
        return;
    }

    let total_xp = claim_local.xp_gained_since_last_coin_minting + gained_xp;
    let num_minted_coins = total_xp / xp_to_mint_hex_coin;

    if num_minted_coins > 0 {
        claim_local.treasury += num_minted_coins;
    }

    claim_local.xp_gained_since_last_coin_minting = total_xp % xp_to_mint_hex_coin;

    ctx.db.claim_local_state().entity_id().update(claim_local);
}

//Having an active instance of this struct will prevent claim_area_around_totem from running game_state_filters::claims_in_radius
pub struct DontCheckAreaAroundClaimSpan {
    initialized: bool,
}

impl DontCheckAreaAroundClaimSpan {
    pub fn start() -> Self {
        DONT_CHECK_AREA_AROUND_CLAIM_COUNTER.replace(DONT_CHECK_AREA_AROUND_CLAIM_COUNTER.get() + 1);
        Self { initialized: true }
    }

    pub fn end(self) {
        // just drop self
    }
}

impl std::ops::Drop for DontCheckAreaAroundClaimSpan {
    fn drop(&mut self) {
        if self.initialized {
            DONT_CHECK_AREA_AROUND_CLAIM_COUNTER.replace(DONT_CHECK_AREA_AROUND_CLAIM_COUNTER.get() - 1);
            self.initialized = false;
        }
    }
}

thread_local! {
    static DONT_CHECK_AREA_AROUND_CLAIM_COUNTER: Cell<i32> = Cell::new(0);
}
