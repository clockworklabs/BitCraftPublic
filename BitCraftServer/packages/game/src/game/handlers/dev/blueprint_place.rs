use crate::game::entities::location::LocationState;
use crate::game::game_state;
use crate::game::handlers::authentication::has_role;
use crate::game::reducer_helpers::building_helpers::{create_building_unsafe, delete_building, DontCreateBuildingSpawnsSpan};
use crate::game::reducer_helpers::loot_chest_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::authentication::Role;
use crate::{game::coordinates::*, messages::components::*};
use crate::{resource_desc, unwrap_or_err, OffsetCoordinatesLarge};
use bitcraft_macro::shared_table_reducer;
use json::JsonValue;
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn blueprint_place(
    ctx: &ReducerContext,
    center: OffsetCoordinatesSmall,
    blueprint_json: String,
    settings_json: String,
    rotation: i32,
    elevation_offset: i16,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    spacetimedb::log::info!("blueprint_place triggered by {}", ctx.sender);

    let center_small = SmallHexTile::from(center);
    let center_large = center_small.parent_large_tile();
    let dimension = center_small.dimension;
    let blueprint = json::parse(blueprint_json.as_str()).unwrap();

    let settings = json::parse(settings_json.as_str()).unwrap();
    let has_terrain = settings["IncludeTerrain"].as_bool().unwrap();
    let has_buildings = settings["IncludeBuildings"].as_bool().unwrap();
    let has_resources = settings["IncludeResources"].as_bool().unwrap();
    let has_pavement = settings["IncludePavement"].as_bool().unwrap();
    let has_small_grid = has_buildings || has_resources || has_pavement;
    let has_large_grid = has_terrain;

    if has_large_grid && !center_small.is_center() {
        return Err("Expected center of large tile".into());
    }

    let clear_entities = has_buildings || has_resources;
    if has_small_grid {
        clear_area(ctx, center_small, rotation, &blueprint, clear_entities, has_pavement);
    }

    //Terrain
    if has_terrain {
        let mut cache = TerrainChunkCache::empty();
        let center_cell = unwrap_or_err!(cache.get_terrain_cell(ctx, &center_large), "Invalid location");
        let center_elevation = center_cell.elevation.max(center_cell.water_level);
        let elevation_offset = elevation_offset + center_elevation;

        for terrain in blueprint["Terrain"].members() {
            let json_coord = &terrain["Coordinates"];
            let coord = (center_large
                + LargeHexTile::from(OffsetCoordinatesLarge {
                    x: json_coord["X"].as_i32().unwrap(),
                    z: json_coord["Z"].as_i32().unwrap(),
                    dimension,
                }))
            .rotate_around(&center_large, rotation);
            let elevation = terrain["Elevation"].as_i16().unwrap();
            let water_level = terrain["WaterLevel"].as_i16().unwrap();
            let is_ground = elevation >= water_level;

            let chunk_coord = ChunkCoordinates::from_terrain_coordinates(coord);
            let terrain_chunk = unwrap_or_err!(cache.get_from_chunk_coordinates(ctx, chunk_coord), "Invalid terrain chunk");
            let chunk_index = terrain_chunk.chunk_index;
            let oc = OffsetCoordinatesLarge::from(coord);
            let mut terrain_cell = unwrap_or_err!(terrain_chunk.get_entity(&oc), "Invalid terrain chunk");
            terrain_cell.elevation = elevation + elevation_offset;
            terrain_cell.water_level = water_level + elevation_offset;
            if (terrain_cell.water_body_type == SurfaceType::Ground as u8) != is_ground {
                terrain_cell.water_body_type = if is_ground {
                    SurfaceType::Ground as u8
                } else {
                    SurfaceType::Lake as u8
                };
            }
            terrain_chunk.set_entity(oc, terrain_cell);
            cache.persist(ctx, chunk_index);
        }
    }

    //Buildings
    if has_buildings {
        let span = DontCreateBuildingSpawnsSpan::start();
        for building in blueprint["Buildings"].members() {
            let json_coord = &building["Coordinates"];
            let coord = (center_small
                + SmallHexTile::from(OffsetCoordinatesSmall {
                    x: json_coord["X"].as_i32().unwrap(),
                    z: json_coord["Z"].as_i32().unwrap(),
                    dimension,
                }))
            .rotate_around(&center_small, rotation);
            let building_desc_id = building["BuildingDescId"].as_i32().unwrap();
            let direction = building["Direction"].as_i32().unwrap();
            let actor_id = game_state::actor_id(&ctx, false)?;
            create_building_unsafe(ctx, actor_id, None, coord, direction, building_desc_id, None)?;
        }
        span.end();
    }

    //Resources
    if has_resources {
        for resource in blueprint["Resources"].members() {
            let json_coord = &resource["Coordinates"];
            let coord = (center_small
                + SmallHexTile::from(OffsetCoordinatesSmall {
                    x: json_coord["X"].as_i32().unwrap(),
                    z: json_coord["Z"].as_i32().unwrap(),
                    dimension,
                }))
            .rotate_around(&center_small, rotation);
            let resource_desc_id = resource["ResourceDescId"].as_i32().unwrap();
            let direction = resource["Direction"].as_i32().unwrap();

            ResourceState::spawn(
                ctx,
                None,
                resource_desc_id,
                coord,
                direction,
                ctx.db.resource_desc().id().find(&resource_desc_id).unwrap().max_health,
                false,
                false,
            )?;
        }
    }

    //Paving
    if has_pavement {
        for paving in blueprint["Pavement"].members() {
            let json_coord = &paving["Coordinates"];
            let coord = (center_small
                + SmallHexTile::from(OffsetCoordinatesSmall {
                    x: json_coord["X"].as_i32().unwrap(),
                    z: json_coord["Z"].as_i32().unwrap(),
                    dimension,
                }))
            .rotate_around(&center_small, rotation);
            let paving_desc_id = paving["PavingDescId"].as_i32().unwrap();

            let entity_id = game_state::create_entity(ctx);
            game_state::insert_location(ctx, entity_id, coord.into());
            ctx.db.paved_tile_state().try_insert(PavedTileState {
                entity_id,
                tile_type_id: paving_desc_id,
                related_entity_id: 0,
            })?;
        }
    }

    spacetimedb::log::info!("Blueprint placed");

    Ok(())
}

fn clear_area(
    ctx: &ReducerContext,
    center_small: SmallHexTile,
    rotation: i32,
    blueprint: &JsonValue,
    delete_entities: bool,
    delete_pavement: bool,
) {
    let dimension = center_small.dimension;
    let area = &blueprint["AreaSmallTiles"];
    for tile in area.members() {
        let coord = (center_small
            + SmallHexTile::from(OffsetCoordinatesSmall {
                x: tile["X"].as_i32().unwrap(),
                z: tile["Z"].as_i32().unwrap(),
                dimension,
            }))
        .rotate_around(&center_small, rotation);

        for location in LocationState::select_all(ctx, &coord) {
            let entity_id = location.entity_id;
            if delete_entities {
                if ctx.db.building_state().entity_id().find(&entity_id).is_some() {
                    delete_building(ctx, 0, entity_id, None, false, false);
                } else if let Some(resource) = ctx.db.resource_state().entity_id().find(&entity_id) {
                    resource.despawn_self(ctx);
                } else if ctx.db.loot_chest_state().entity_id().find(&entity_id).is_some() {
                    loot_chest_helpers::delete_loot_chest(ctx, entity_id);
                }
            }

            if delete_pavement && ctx.db.paved_tile_state().entity_id().find(&entity_id).is_some() {
                PavedTileState::delete_paving(ctx, &entity_id);
            }
        }
    }
}
