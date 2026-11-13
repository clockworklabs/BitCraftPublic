use spacetimedb::log;
use spacetimedb::ReducerContext;
use spacetimedb::Table;

use crate::footprint_tile_state;
use crate::game::coordinates::LargeHexTile;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::location_state;
use crate::resource_desc;
use crate::utils::iter_utils::GroupByAndCount;
use crate::{
    game::{coordinates::SmallHexTile, game_state},
    messages::util::SmallHexTileMessage,
    BuildingDesc, FootprintType, ResourceDesc,
};

use crate::messages::components::{FootprintTileState, ResourceState};

pub fn create_resource_footprint(ctx: &ReducerContext, entity_id: u64, resource: &ResourceDesc, direction_index: i32) {
    let location = ctx.db.location_state().entity_id().find(entity_id).unwrap();
    let coordinates = location.coordinates();
    let footprint = resource.get_footprint(&coordinates, direction_index);

    for (coords, footprint_type) in footprint {
        // insert footprint component
        let new_entity_id = game_state::create_entity(ctx);
        if ctx
            .db
            .footprint_tile_state()
            .try_insert(FootprintTileState {
                entity_id: new_entity_id,

                footprint_type,
                owner_entity_id: entity_id,
            })
            .is_err()
        {
            log::error!("Failed to insert footprint");
        }
        let offset = coords.to_offset_coordinates();
        game_state::insert_location(ctx, new_entity_id, offset);
    }
}

pub fn create_project_site_footprint(ctx: &ReducerContext, entity_id: u64, footprint: &Vec<(SmallHexTile, FootprintType)>) {
    create_footprint(ctx, entity_id, footprint, &Some(project_site_override_footprint_type));
}

fn project_site_override_footprint_type(footprint_type: &FootprintType) -> FootprintType {
    match footprint_type {
        FootprintType::Hitbox => FootprintType::Walkable,
        _ => *footprint_type,
    }
}

pub fn update_footprint_after_building_completion(ctx: &ReducerContext, entity_id: u64, direction: i32, building: &BuildingDesc) {
    let location = ctx.db.location_state().entity_id().find(entity_id).unwrap();
    let coordinates = location.coordinates();
    let footprint = building.get_footprint(&coordinates, direction);
    update_footprint_after_completion(ctx, entity_id, footprint);
}

pub fn update_footprint_after_resource_completion(
    ctx: &ReducerContext,
    entity_id: u64,
    coordinates: SmallHexTile,
    direction: i32,
    resource: &ResourceDesc,
) {
    let footprint = resource.get_footprint(&coordinates, direction);
    update_footprint_after_completion(ctx, entity_id, footprint);
}

fn update_footprint_after_completion(ctx: &ReducerContext, entity_id: u64, footprint: Vec<(SmallHexTileMessage, FootprintType)>) {
    // get existing fooptrint and sort by entity_id, since that is the order in which it was inserted
    let mut existing_footprint: Vec<FootprintTileState> = ctx.db.footprint_tile_state().owner_entity_id().filter(entity_id).collect();
    existing_footprint.sort_by(|a, b| a.entity_id.partial_cmp(&b.entity_id).unwrap());

    // Loop over building footprint description, and update based on the order
    for (index, (_, footprint_type)) in footprint.iter().enumerate() {
        let existing = &existing_footprint[index];
        if existing.footprint_type == *footprint_type {
            continue;
        }

        ctx.db.footprint_tile_state().entity_id().update(FootprintTileState {
            entity_id: existing.entity_id,
            footprint_type: *footprint_type,
            owner_entity_id: existing.owner_entity_id,
        });
    }
}

pub fn create_footprint(
    ctx: &ReducerContext,
    entity_id: u64,
    footprint: &Vec<(SmallHexTile, FootprintType)>,
    override_type_fn: &Option<fn(&FootprintType) -> FootprintType>,
) {
    for (coords, footprint_type) in footprint {
        // insert footprint component
        let new_entity_id = game_state::create_entity(ctx);
        if ctx
            .db
            .footprint_tile_state()
            .try_insert(FootprintTileState {
                entity_id: new_entity_id,

                footprint_type: match override_type_fn {
                    Some(override_type_fn) => override_type_fn(footprint_type),
                    None => *footprint_type,
                },
                owner_entity_id: entity_id,
            })
            .is_err()
        {
            log::error!("Failed to insert footprint");
        }

        let offset = coords.to_offset_coordinates();
        game_state::insert_location(ctx, new_entity_id, offset);
    }
}

pub fn clear_resources_under_footprint(ctx: &ReducerContext, footprint: &[(SmallHexTile, FootprintType)], clear_all: bool) {
    for (coords, _) in footprint {
        // destroy resources under footprint
        if let Some(resource) = ResourceState::get_at_location(ctx, coords) {
            let resource_desc = ctx.db.resource_desc().id().find(resource.resource_id).unwrap();
            if resource_desc.flattenable || clear_all {
                resource.despawn_self(ctx);
            }
        }
    }
}

pub fn delete_footprint(ctx: &ReducerContext, entity_id: u64) {
    let footprints = ctx.db.footprint_tile_state().owner_entity_id().filter(entity_id);
    for footprint in footprints {
        ctx.db.footprint_tile_state().entity_id().delete(footprint.entity_id);
        ctx.db.location_state().entity_id().delete(footprint.entity_id);
    }
}

pub fn clear_and_flatten_terrain_under_footprint(
    ctx: &ReducerContext,
    footprint: &[(SmallHexTile, FootprintType)],
    flatten_terrain: bool,
) {
    //Remove resources
    clear_resources_under_footprint(ctx, footprint, true);

    if !flatten_terrain {
        return;
    }

    //Compute unique large coords
    let mut large_tiles: Vec<LargeHexTile> = Vec::with_capacity(footprint.len());
    for (tile, _) in footprint {
        if tile.is_corner() {
            for lt in tile.get_terrain_coordinates() {
                if !large_tiles.contains(&lt) {
                    large_tiles.push(lt);
                }
            }
        } else {
            let lt = tile.parent_large_tile();
            if !large_tiles.contains(&lt) {
                large_tiles.push(lt);
            }
        }
    }

    //Fetch elevation for all tiles
    let mut terrain_cache = TerrainChunkCache::empty();
    let mut tile_elevation: Vec<i16> = Vec::with_capacity(large_tiles.len());
    for lt in &large_tiles {
        if let Some(tc) = terrain_cache.get_terrain_cell(ctx, lt) {
            tile_elevation.push(tc.elevation.max(tc.water_level));
        }
    }

    //Find the most common elevation
    let mut elevation_count: Vec<(i16, usize)> = tile_elevation
        .iter()
        .group_by_and_count(|e| **e)
        .iter()
        .map(|v| (*v.0, *v.1))
        .collect();
    elevation_count.sort_by_key(|b| std::cmp::Reverse(b.1));
    let target_elevation = elevation_count[0].0;

    //Set elevation for all tiles
    for lt in large_tiles {
        if let Some(tc) = terrain_cache.get_from_chunk_coordinates(ctx, lt.chunk_coordinates()) {
            let i = tc.get_index(lt);
            tc.elevations[i] = target_elevation;
            tc.water_levels[i] = 0;
        }
    }
    terrain_cache.consume_and_persist_all(ctx);
}
