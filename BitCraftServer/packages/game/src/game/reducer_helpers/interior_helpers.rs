use spacetimedb::{ReducerContext, Table};
use std::collections::HashMap;

use spacetimedb::{log, rand::Rng};

use crate::{
    building_desc, building_portal_desc, building_state, dimension_description_state, dimension_network_state, enemy_state,
    game::{
        coordinates::*,
        game_state::{self, game_state_filters},
        handlers::server::{
            enemy_despawn,
            interior_set_collapsed::{interior_set_collapsed_timer, InteriorSetCollapsedTimer},
        },
    },
    herd_state, interior_collapse_trigger_state, interior_instance_desc, interior_network_desc, interior_portal_connections_desc,
    interior_shape_desc, interior_spawn_desc, loot_chest_state,
    messages::{
        components::{
            combat_dimension_state, deployable_state_v2, dungeon_state, CombatDimensionState, DungeonState, InteriorPlayerCountState,
            LostItemsState, MobileEntityState,
        },
        game_util::DimensionType,
    },
    portal_state, resource_clump_desc, resource_desc, resource_state, terrain_chunk_state, unwrap_or_err, BuildingPortalDesc,
    FootprintTile, FootprintType, InteriorPortalConnectionsDesc, InteriorSpawnDesc, InteriorSpawnType, LocationState, TerrainCell,
};

use super::{
    building_helpers::{create_building_component, create_building_footprint, create_building_spawns, delete_building},
    loot_chest_helpers,
    timer_helpers::now_plus_secs,
};

use crate::messages::components::{
    DimensionDescriptionState, DimensionNetworkState, HerdState, InteriorCollapseTriggerState, NpcState, PortalState, ResourceState,
    TerrainChunkState,
};

pub fn interior_trigger_collapse(ctx: &ReducerContext, dimension_network_entity_id: u64) -> Result<(), String> {
    let mut dimension_network = unwrap_or_err!(
        ctx.db.dimension_network_state().entity_id().find(&dimension_network_entity_id),
        "Invalid dimension network"
    );
    if dimension_network.collapse_respawn_timestamp > game_state::unix_ms(ctx.timestamp) {
        //Already collapsing or respawning
        return Ok(());
    }

    let building = ctx.db.building_state().entity_id().find(&dimension_network.building_id).unwrap();
    let interior_descriptor = ctx
        .db
        .interior_network_desc()
        .building_id()
        .find(&building.building_description_id)
        .unwrap();
    let collapse_timestamp = game_state::unix_ms(ctx.timestamp) + (interior_descriptor.trigger_collapse_time as u64 * 1000);

    let dimensions: Vec<DimensionDescriptionState> = ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(dimension_network_entity_id)
        .collect();

    for mut dimension in dimensions {
        dimension.collapse_timestamp = collapse_timestamp;
        ctx.db.dimension_description_state().entity_id().update(dimension);
    }

    dimension_network.collapse_respawn_timestamp = collapse_timestamp;
    ctx.db.dimension_network_state().entity_id().update(dimension_network);

    ctx.db
        .interior_set_collapsed_timer()
        .try_insert(InteriorSetCollapsedTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(interior_descriptor.trigger_collapse_time as u64, ctx.timestamp),
            dimension_network_entity_id,
            is_collapsed: true,
        })
        .ok()
        .unwrap();

    Ok(())
}

pub fn find_teleport_coordinates_for_interior_destruction(ctx: &ReducerContext, building_id: u64) -> OffsetCoordinatesFloat {
    let building = ctx.db.building_state().entity_id().find(&building_id).unwrap();
    let building_coord = game_state_filters::coordinates(ctx, building_id);
    let building_portals: Vec<BuildingPortalDesc> = ctx
        .db
        .building_portal_desc()
        .building_id()
        .filter(building.building_description_id)
        .collect();

    let mut teleport_destination = building_coord;
    for portal in building_portals {
        let coord = building_coord
            + SmallHexTile {
                x: portal.pos_x,
                z: portal.pos_z,
                dimension: 0,
            };
        let coord_rotated = coord.rotate_around(&building_coord, building.direction_index / 2);
        if !game_state_filters::has_hitbox_footprint(ctx, coord_rotated) {
            teleport_destination = coord_rotated;
            break;
        }
    }
    return OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(teleport_destination));
}

pub fn delete_dimension_network(
    ctx: &ReducerContext,
    dimension_network_entity_id: u64,
    interior_teleport_destination: OffsetCoordinatesFloat,
) {
    let interior_teleport_destination = Some(interior_teleport_destination);

    let dimensions: Vec<(u32, u64)> = ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(dimension_network_entity_id)
        .map(|a| (a.dimension_id, a.entity_id))
        .collect();

    for dimension in dimensions {
        ctx.db.combat_dimension_state().dimension_id().delete(dimension.0);

        let entities: Vec<u64> = LocationState::select_all_in_interior_dimension_iter(ctx, dimension.0)
            .map(|a| a.entity_id)
            .collect();

        for entity in &entities {
            if let Some(_) = ctx.db.building_state().entity_id().find(entity) {
                delete_building(ctx, 0, *entity, interior_teleport_destination.clone(), true, false);
            }
        }
        for entity in &entities {
            if let Some(resource) = ctx.db.resource_state().entity_id().find(entity) {
                resource.despawn_self(ctx);
            }
        }
        for entity in &entities {
            if let Some(_) = ctx.db.loot_chest_state().entity_id().find(entity) {
                loot_chest_helpers::delete_loot_chest(ctx, *entity);
            }
        }

        // Interior deployables are subject to "lost and found"
        let overworld_location: OffsetCoordinatesSmall = interior_teleport_destination.unwrap().into();
        for mobile_entity in MobileEntityState::select_all_in_interior_dimension_iter(ctx, dimension.0) {
            if let Some(deployable) = ctx.db.deployable_state_v2().entity_id().find(mobile_entity.entity_id) {
                LostItemsState::generate_lost_items_for_deployable(ctx, &deployable, overworld_location.into());
            }
        }

        ctx.db.dimension_description_state().entity_id().delete(&dimension.1);
    }

    ctx.db.dimension_network_state().entity_id().delete(&dimension_network_entity_id);
    ctx.db.dungeon_state().entity_id().delete(&dimension_network_entity_id);
}

pub fn respawn_interior(ctx: &ReducerContext, dimension_network_description_id: u64) {
    //All ops processed by this method will not be broadcast since it's called from interior_set_collapsed which is ignored by game_client_subscription_manager
    //This doesn't matter though as there should be no players in these dimensions
    let dimensions: Vec<DimensionDescriptionState> = ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(dimension_network_description_id)
        .collect();

    for dimension in &dimensions {
        let interior_instance = ctx.db.interior_instance_desc().id().find(&dimension.interior_instance_id).unwrap();
        let shape = ctx
            .db
            .interior_shape_desc()
            .id()
            .find(&interior_instance.interior_shape_id)
            .unwrap();
        let spawns: Vec<InteriorSpawnDesc> = ctx
            .db
            .interior_spawn_desc()
            .interior_instance_id()
            .filter(dimension.interior_instance_id)
            .collect();

        for spawn in spawns {
            if !spawn.respawn {
                continue;
            }
            let x = spawn.spawn_x - shape.min_x;
            let z = spawn.spawn_z - shape.min_z;
            match spawn.spawn_type {
                InteriorSpawnType::Building => {
                    //Assumes no portal buildings respawn
                    let location = OffsetCoordinatesSmall {
                        x,
                        z,
                        dimension: dimension.dimension_id,
                    };
                    if let Some(building_id) = game_state_filters::building_id_at_coordinates(ctx, &SmallHexTile::from(location)) {
                        delete_building(ctx, 0, building_id, None, false, false);
                        //No players inside so no produced requests
                    }

                    let entity_id = spawn_interior_building(ctx, &spawn, location);
                    add_interior_collapse_trigger(ctx, &spawn, entity_id, dimension_network_description_id);
                }

                InteriorSpawnType::Resource => {
                    let location = OffsetCoordinatesSmall {
                        x,
                        z,
                        dimension: dimension.dimension_id,
                    };
                    spawn_interior_resource(ctx, &spawn, location, dimension_network_description_id);
                    //add_interior_collapse_trigger(state, spawn, entity_id, dimension_network_descriptor_id); // Handled by spawn_interior_resource
                }

                InteriorSpawnType::Chest => {
                    let spawn_location = OffsetCoordinatesSmall {
                        x,
                        z,
                        dimension: dimension.dimension_id,
                    };
                    if let Some(building_id) = game_state_filters::building_id_at_coordinates(ctx, &SmallHexTile::from(spawn_location)) {
                        loot_chest_helpers::delete_loot_chest(ctx, building_id);
                    }
                    let entity_id = game_state::create_entity(ctx);
                    let r = loot_chest_helpers::spawn_loot_chest(
                        ctx,
                        &spawn.loot_chests,
                        entity_id,
                        spawn_location.into(),
                        spawn.direction,
                        0,
                        spawn.id,
                        false,
                    );
                    if let Err(err) = r {
                        panic!("{}", err);
                    }
                    add_interior_collapse_trigger(ctx, &spawn, entity_id, dimension_network_description_id);
                }

                InteriorSpawnType::Enemy => {
                    let coord = SmallHexTile::from(OffsetCoordinatesSmall {
                        x,
                        z,
                        dimension: dimension.dimension_id,
                    });
                    if let Some(herd_location) = LocationState::select_all(ctx, &coord)
                        .filter(|loc| ctx.db.herd_state().entity_id().find(&loc.entity_id).is_some())
                        .next()
                    {
                        // Despawn all enemies from that herd
                        for despawned_enemy in ctx.db.enemy_state().herd_entity_id().filter(herd_location.entity_id) {
                            enemy_despawn::reduce(ctx, despawned_enemy.entity_id);
                        }
                        // Despawn Herd
                        ctx.db.herd_state().entity_id().delete(herd_location.entity_id);
                        // Respawn Herd
                        let herd = HerdState::new(ctx, spawn.traveler_ruin_entity_id); // [MIGRATION TODO] traveler_ruin_entity_id is in reality enemy_ai_param_desc_id
                        let herd_entity_id = herd.entity_id;
                        if let Err(err) = ctx.db.herd_state().try_insert(herd) {
                            log::error!("{}", err);
                        }
                        game_state::insert_location(ctx, herd_entity_id, herd_location.offset_coordinates());

                        // Collapse trigger is now on the herd, and evaluated when the herd reaches 0 population
                        add_interior_collapse_trigger(ctx, &spawn, herd_entity_id, dimension_network_description_id);
                    }
                }

                InteriorSpawnType::Traveler => {} // Interior Travelers stay forever and ever
                _ => panic!("Unexpected interior spawn: {}", spawn.id),
            };
        }
    }
}

pub fn create_player_interior(ctx: &ReducerContext, building_description_id: i32, external_building_entity_id: u64) -> Result<u64, String> {
    let building_state = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&external_building_entity_id),
        "Building does not exist"
    );

    let dimension_network_descriptor_id = game_state::create_entity(ctx);
    create_building_interior_internal(
        ctx,
        building_description_id,
        external_building_entity_id,
        dimension_network_descriptor_id,
        building_state.direction_index,
        0,
        false,
    )?;
    Ok(dimension_network_descriptor_id)
}

pub fn create_building_interior(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    let building_state = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id),
        "Building does not exist"
    );
    let dimension_network_descriptor_id = building_entity_id;
    create_building_interior_internal(
        ctx,
        building_state.building_description_id,
        building_entity_id,
        dimension_network_descriptor_id,
        building_state.direction_index,
        building_state.claim_entity_id,
        true,
    )
}

fn create_building_interior_internal(
    ctx: &ReducerContext,
    building_description_id: i32,
    external_building_entity_id: u64,
    dimension_network_descriptor_id: u64,
    direction_index: i32,
    claim_entity_id: u64,
    update_external_portals: bool,
) -> Result<(), String> {
    let interior_desc = match ctx.db.interior_network_desc().building_id().find(building_description_id) {
        Some(ind) => ind,
        None => return Ok(()),
    };
    let mut dimension_map: HashMap<i32, u32> = HashMap::new(); //Links interior_desc.child_interior_instances to state.dimension_descriptions
    let mut spawn_map: HashMap<i32, u64> = HashMap::new(); //Links spawns to entities
    let mut interior_shape_map: HashMap<u32, i32> = HashMap::new(); //Links newly-created dimensions to interior shape ids

    for interior_instance in &interior_desc.child_interior_instances {
        //Create dimension descriptor
        let dimension = game_state::create_dimension(ctx) as u32;
        let dimension_entity = game_state::create_entity(ctx);
        let dimension_desc = DimensionDescriptionState {
            entity_id: dimension_entity,
            dimension_id: dimension,
            dimension_type: interior_desc.dimension_type,
            interior_instance_id: *interior_instance,
            dimension_position_large_x: 0,
            dimension_position_large_z: 0,
            dimension_size_large_x: 1,
            dimension_size_large_z: 1,
            dimension_network_entity_id: dimension_network_descriptor_id,
            collapse_timestamp: 0,
        };
        if let Err(error) = ctx.db.dimension_description_state().try_insert(dimension_desc) {
            return Err(format!("Failed to insert dimension description: {{0}}|~{}", error));
        }
        dimension_map.insert(*interior_instance, dimension);
        let interior_instance_desc = ctx.db.interior_instance_desc().id().find(*interior_instance).unwrap();

        //Add terrain (a flat chunk)
        let mut chunk = TerrainChunkState::default_with_capacity();
        chunk.dimension = dimension;
        chunk.chunk_index = ChunkCoordinates { x: 0, z: 0, dimension }.chunk_index();
        for x in 0..TerrainChunkState::WIDTH {
            for z in 0..TerrainChunkState::HEIGHT {
                let cell = TerrainCell {
                    x: x as i32,
                    z: z as i32,
                    dimension,
                    elevation: 20,
                    biomes: interior_instance_desc.biome as u32,
                    biome_density: 127,
                    ..Default::default()
                };
                chunk.set_entity(
                    OffsetCoordinatesLarge {
                        x: x as i32,
                        z: z as i32,
                        dimension,
                    },
                    cell,
                );
            }
        }
        if ctx.db.terrain_chunk_state().try_insert(chunk).is_err() {
            return Err("Failed to insert terrain chunk".into());
        }

        let instance = ctx.db.interior_instance_desc().id().find(interior_instance).unwrap();
        let shape = ctx.db.interior_shape_desc().id().find(&instance.interior_shape_id).unwrap();
        interior_shape_map.insert(dimension, instance.interior_shape_id);

        let mut combat_dimension_exists = ctx.db.combat_dimension_state().dimension_id().find(dimension).is_some();

        //Add spawns
        for spawn in ctx.db.interior_spawn_desc().interior_instance_id().filter(interior_instance) {
            let x = spawn.spawn_x - shape.min_x;
            let z = spawn.spawn_z - shape.min_z;
            match spawn.spawn_type {
                InteriorSpawnType::Building => {
                    let location = OffsetCoordinatesSmall { x, z, dimension };
                    let entity_id = spawn_interior_building(ctx, &spawn, location);
                    spawn_map.insert(spawn.id, entity_id);
                    add_interior_collapse_trigger(ctx, &spawn, entity_id, dimension_network_descriptor_id);
                }

                InteriorSpawnType::Resource => {
                    let location = OffsetCoordinatesSmall { x, z, dimension };
                    spawn_interior_resource(ctx, &spawn, location, dimension_network_descriptor_id);
                    //add_interior_collapse_trigger(state, spawn, entity_id, dimension_network_descriptor_id); // Handled by spawn_interior_resource
                }

                InteriorSpawnType::Chest => {
                    let spawn_location = OffsetCoordinatesSmall { x, z, dimension };
                    let entity_id = game_state::create_entity(ctx);
                    loot_chest_helpers::spawn_loot_chest(
                        ctx,
                        &spawn.loot_chests,
                        entity_id,
                        spawn_location.into(),
                        spawn.direction,
                        0,
                        spawn.id,
                        false,
                    )?;
                    add_interior_collapse_trigger(ctx, &spawn, entity_id, dimension_network_descriptor_id);
                }
                InteriorSpawnType::Enemy => {
                    if !combat_dimension_exists {
                        ctx.db
                            .combat_dimension_state()
                            .insert(CombatDimensionState { dimension_id: dimension });
                        combat_dimension_exists = true;
                    }

                    // create a new herd for that enemy entry. It will be used for respawn
                    // [MIGRATION TODO] we WILL want to add an EnemyAIParams id instead of an enemy type in InteriorSpawns.
                    let spawn_location = OffsetCoordinatesSmall { x, z, dimension };

                    let herd = HerdState::new(ctx, spawn.traveler_ruin_entity_id); // [MIGRATION TODO] traveler_ruin_entity_id is in reality enemy_ai_param_desc_id
                    let herd_entity_id = herd.entity_id;
                    if let Err(err) = ctx.db.herd_state().try_insert(herd) {
                        log::error!("{}", err);
                    }
                    game_state::insert_location(ctx, herd_entity_id, spawn_location);

                    // Collapse trigger is now on the herd, and evaluated when the herd reaches 0 population
                    add_interior_collapse_trigger(ctx, &spawn, herd_entity_id, dimension_network_descriptor_id);
                }
                InteriorSpawnType::Traveler => {
                    let spawn_location = OffsetCoordinatesSmall { x, z, dimension };
                    NpcState::spawn_with_ruins(
                        ctx,
                        spawn.traveler_type,
                        spawn.traveler_ruin_entity_id,
                        spawn.direction,
                        spawn_location,
                        false,
                    );
                }
                _ => panic!("Unexpected interior spawn: {}", spawn.id),
            };
        }
    }

    //Link external building portals to spawns
    let mut entrance_dimension = 0;
    let external_portals: Vec<BuildingPortalDesc> = ctx
        .db
        .building_portal_desc()
        .building_id()
        .filter(building_description_id)
        .collect();
    for portal in external_portals {
        let connections: Vec<InteriorPortalConnectionsDesc> = ctx
            .db
            .interior_portal_connections_desc()
            .entrance_portal_id()
            .filter(portal.id)
            .collect();
        for connection in connections {
            let connected_portal = ctx.db.building_portal_desc().id().find(&connection.exit_portal_id).unwrap();
            let connected_spawn = ctx.db.interior_spawn_desc().id().find(&connection.exit_spawn_id).unwrap();
            let connected_dimension = dimension_map.get(&connected_spawn.interior_instance_id).unwrap();
            if update_external_portals {
                let shape = ctx
                    .db
                    .interior_shape_desc()
                    .id()
                    .find(interior_shape_map.get(connected_dimension).unwrap())
                    .unwrap();
                let portal_hex = SmallHexTile {
                    x: connected_portal.pos_x,
                    z: connected_portal.pos_z,
                    dimension: 0,
                }
                .rotate_around(&SmallHexTile { x: 0, z: 0, dimension: 0 }, direction_index / 2);
                let portal_oc = OffsetCoordinatesSmall::from(portal_hex);
                let target_x = connected_spawn.spawn_x + portal_oc.x - shape.min_x;
                let target_z = connected_spawn.spawn_z + portal_oc.z - shape.min_z;
                let portal = PortalState {
                    entity_id: external_building_entity_id,
                    destination_dimension: *connected_dimension,
                    destination_x: target_x,
                    destination_z: target_z,
                    enabled: true,
                    allow_deployables: connected_portal.allow_deployables,
                    target_building_entity_id: 0, //Can't move spawns
                };
                if ctx.db.portal_state().try_insert(portal).is_err() {
                    return Err("Failed to insert portal".into());
                }
            }
            entrance_dimension = *connected_dimension;
        }
    }

    //Link spawn portals
    for building in spawn_map {
        let connections: Vec<InteriorPortalConnectionsDesc> = ctx
            .db
            .interior_portal_connections_desc()
            .entrance_spawn_id()
            .filter(building.0)
            .collect();
        for connection in connections {
            let connected_portal = ctx.db.building_portal_desc().id().find(&connection.exit_portal_id).unwrap();
            if connection.exit_spawn_id != 0 {
                //Connect to another spawn
                let connected_spawn = ctx.db.interior_spawn_desc().id().find(&connection.exit_spawn_id).unwrap();
                let connected_dimension = dimension_map.get(&connected_spawn.interior_instance_id).unwrap();
                let shape = ctx
                    .db
                    .interior_shape_desc()
                    .id()
                    .find(interior_shape_map.get(connected_dimension).unwrap())
                    .unwrap();
                let target_x = connected_spawn.spawn_x - shape.min_x;
                let target_z = connected_spawn.spawn_z - shape.min_z;
                let target_hex = SmallHexTile::from(OffsetCoordinatesSmall {
                    x: target_x,
                    z: target_z,
                    dimension: 0,
                });
                let portal_hex = target_hex
                    + SmallHexTile {
                        x: connected_portal.pos_x,
                        z: connected_portal.pos_z,
                        dimension: 0,
                    };
                let rotated = portal_hex.rotate_around(&target_hex, connected_spawn.direction / 2);
                let rotated_oc = OffsetCoordinatesSmall::from(rotated);
                let portal = PortalState {
                    entity_id: building.1,
                    destination_dimension: *connected_dimension,
                    destination_x: rotated_oc.x,
                    destination_z: rotated_oc.z,
                    enabled: true,
                    allow_deployables: connected_portal.allow_deployables,
                    target_building_entity_id: 0, //Can't move spawns
                };
                if ctx.db.portal_state().try_insert(portal).is_err() {
                    return Err("Failed to insert portal".into());
                }
            } else {
                //Connect to building exterior
                let building_coord = game_state_filters::coordinates(ctx, external_building_entity_id).to_offset_coordinates();
                let target_x = building_coord.x;
                let target_z = building_coord.z;
                let target_hex = SmallHexTile::from(OffsetCoordinatesSmall {
                    x: target_x,
                    z: target_z,
                    dimension: 0,
                });
                let portal_hex = target_hex
                    + SmallHexTile {
                        x: connected_portal.pos_x,
                        z: connected_portal.pos_z,
                        dimension: 0,
                    };
                let rotated = portal_hex.rotate_around(&SmallHexTile::from(building_coord), direction_index / 2);
                let rotated_oc = OffsetCoordinatesSmall::from(rotated);
                let portal = PortalState {
                    entity_id: building.1,
                    destination_dimension: building_coord.dimension,
                    destination_x: rotated_oc.x,
                    destination_z: rotated_oc.z,
                    enabled: true,
                    allow_deployables: connected_portal.allow_deployables,
                    target_building_entity_id: external_building_entity_id,
                };
                if ctx.db.portal_state().try_insert(portal).is_err() {
                    return Err("Failed to insert portal".into());
                }
            }
        }
    }

    let network_state = DimensionNetworkState {
        entity_id: dimension_network_descriptor_id,
        building_id: dimension_network_descriptor_id, // This has the same value as the external building except for player housing where it's not used (but key is unique so it needs a value)
        entrance_dimension_id: entrance_dimension,
        collapse_respawn_timestamp: 0,
        is_collapsed: false,
        rent_entity_id: 0,
        claim_entity_id: claim_entity_id,
    };

    if interior_desc.dimension_type == DimensionType::Dungeon {
        ctx.db.dungeon_state().insert(DungeonState {
            entity_id: external_building_entity_id,
            location: game_state_filters::offset_coordinates(ctx, external_building_entity_id),
        });
        InteriorPlayerCountState::create(ctx, &network_state);
    }

    if ctx.db.dimension_network_state().try_insert(network_state).is_err() {
        return Err("Failed to insert dimension network description".into());
    }

    Ok(())
}

fn spawn_interior_building(ctx: &ReducerContext, spawn: &InteriorSpawnDesc, location: OffsetCoordinatesSmall) -> u64 {
    let building_description = ctx.db.building_desc().id().find(&spawn.building_id).unwrap();
    let entity_id = game_state::create_entity(ctx);
    game_state::insert_location(ctx, entity_id, location);

    create_building_component(ctx, 0, entity_id, spawn.direction, &building_description, 0);

    create_building_footprint(ctx, entity_id, spawn.direction, &building_description, &None);

    //if create_building_claim(state, static_data::* entity_id, true).is_err() {
    //    log::error!("Failed to create building claim");
    //    return "Failed to create building claim".into();
    //}

    create_building_spawns(ctx, entity_id);

    entity_id
}

pub(crate) fn spawn_interior_resource(
    ctx: &ReducerContext,
    spawn: &InteriorSpawnDesc,
    location: OffsetCoordinatesSmall,
    dimension_network_descriptor_id: u64,
) {
    let clump = ctx.db.resource_clump_desc().id().find(&spawn.resource_clump_id).unwrap();

    let hex_coordinates = SmallHexTile::from(location);
    let facing_direction = HexDirection::from(spawn.direction);
    let default_footprint = vec![FootprintTile {
        x: 0,
        z: 0,
        footprint_type: FootprintType::WalkableResource,
    }];
    for i in 0..clump.resource_id.len() {
        let resource_id = clump.resource_id[i];
        let offset_x = clump.x[i];
        let offset_z = clump.z[i];
        let resource_dir = clump.direction[i];

        let mut res_footprint = &ctx.db.resource_desc().id().find(&resource_id).unwrap().footprint;
        if res_footprint.len() == 0 {
            // default: single-tile hitbox resource
            res_footprint = &default_footprint
        }
        for res_footprint_delta in res_footprint.iter().filter(|f| f.footprint_type != FootprintType::Perimeter) {
            let mut delta = res_footprint_delta.clone();
            let is_center = res_footprint_delta.x == 0 && res_footprint_delta.z == 0;
            delta.x += offset_x;
            delta.z += offset_z;
            let taken_coordinates = (SmallHexTile {
                x: hex_coordinates.x + delta.x,
                z: hex_coordinates.z + delta.z,
                dimension: hex_coordinates.dimension,
            })
            .rotate_around(&hex_coordinates, (facing_direction as i32) / 2);
            let direction = if resource_dir == -1 {
                HexDirection::FLAT[ctx.rng().gen_range(0..HexDirection::FLAT.len()) as usize] as i32
            } else {
                let mut dir = facing_direction;
                for _ in 0..resource_dir {
                    dir = HexDirection::next_flat(dir);
                }
                dir as i32
            };

            if is_center {
                let entity_id = game_state::create_entity(ctx);
                ResourceState::spawn(
                    ctx,
                    Some(entity_id),
                    resource_id,
                    taken_coordinates,
                    direction,
                    ctx.db.resource_desc().id().find(&resource_id).unwrap().max_health,
                    false,
                    true,
                )
                .unwrap();
                add_interior_collapse_trigger(ctx, spawn, entity_id, dimension_network_descriptor_id);
            }
        }
    }
}

fn add_interior_collapse_trigger(ctx: &ReducerContext, spawn: &InteriorSpawnDesc, entity_id: u64, dimension_network_descriptor_id: u64) {
    if spawn.collapse_trigger {
        if ctx
            .db
            .interior_collapse_trigger_state()
            .try_insert(InteriorCollapseTriggerState {
                entity_id,
                dimension_network_entity_id: dimension_network_descriptor_id,
            })
            .is_err()
        {
            log::error!("Failed to insert interior collapse trigger");
        }
    }
}
