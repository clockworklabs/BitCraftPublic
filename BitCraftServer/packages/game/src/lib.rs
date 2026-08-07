pub mod agents;
pub mod game;
pub mod i18n;
pub mod import_reducers;
pub mod inter_module;
pub mod macros;
pub mod messages;
pub mod table_caches;
pub mod utils;

use crate::game::claim_helper::DontCheckAreaAroundClaimSpan;
use crate::game::coordinates::*;
use crate::game::game_state::{self, create_entity, insert_location};
use crate::game::handlers::authentication::has_role;
use crate::messages::authentication::{developer, IdentityRole, Role, ServerIdentity};
use crate::messages::generic::{AdminBroadcast, ResourceCount};
use crate::messages::generic::{Config, Globals};
use crate::messages::world_gen::WorldGenWorldDefinition;
use bitcraft_macro::shared_table_reducer;
use game::dimensions;
use game::handlers::authentication::is_authenticated;
use game::handlers::player::sign_out::sign_out;
use game::reducer_helpers::building_helpers::{
    create_building_claim, create_building_component, create_building_footprint, create_building_spawns,
};
use game::reducer_helpers::interior_helpers::create_building_interior;
use game::world_gen::resources_log::{resources_log, ResourcesLog};
use game::world_gen::world_definition::WorldDefinition;
use game::world_gen::world_generation::world_graph::WorldGraph;
use game::world_gen::world_generator::{self, GeneratedWorld};
use game::world_gen::{dev_island, flat_world};
use messages::authentication::{blocked_identity, identity_role};
use messages::generic::{
    admin_broadcast, config, globals, region_control_info, resource_count, world_region_name_state, world_region_state, RegionControlInfo,
    WorldRegionNameState, WorldRegionState,
};
use messages::world_gen::{WorldGenGeneratedBuilding, WorldGenGeneratedResourceDeposit};
use region_coordinates::RegionCoordinates;
use spacetimedb::{log, ReducerContext, Table};

use crate::game::location_cache::*;
use crate::messages::components::*;
use crate::messages::static_data::*;

#[spacetimedb::reducer(init)]
pub fn initialize(ctx: &ReducerContext) -> Result<(), String> {
    match ctx.db.config().version().find(&0) {
        Some(config) => {
            if config.env != "dev" {
                // This check is to prevent access to this reducer after the db has been initialized.
                if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
                    return Err("Caller is not the owner of the database".into());
                }
            }
        }
        _ => {}
    };

    if ctx
        .db
        .identity_role()
        .try_insert(IdentityRole {
            role: Role::Admin,
            identity: ctx.sender,
        })
        .is_err()
    {
        log::error!("Failed to insert owner identity");
    }
    if ctx
        .db
        .identity_role()
        .try_insert(IdentityRole {
            role: Role::Admin,
            identity: ctx.identity(),
        })
        .is_err()
    {
        log::error!("Failed to insert database identity");
    }

    ServerIdentity::set(&ctx);

    if ctx
        .db
        .admin_broadcast()
        .try_insert(AdminBroadcast {
            version: 0,
            title: String::new(),
            message: String::new(),
            sign_out: false,
            timestamp: ctx.timestamp,
        })
        .is_err()
    {
        log::error!("Failed to insert AdminBroadcast");
    }

    if ctx
        .db
        .globals()
        .try_insert(Globals {
            version: 0,
            entity_pk_counter: 1, //0 == overworld dimension description
            dimension_counter: 1,
            region_index: 0,
        })
        .is_err()
    {
        log::error!("Failed to insert globals");
    }

    if ctx
        .db
        .config()
        .try_insert(Config {
            version: 0,
            env: "dev".to_string(), // by default, a new node will be set to "dev" so we can upload its config independently of authorizations
            agents_enabled: false,
        })
        .is_err()
    {
        log::error!("Failed to insert config");
    }

    log::info!("Initialized bitcraft spacetimedb.");
    Ok(())
}

// Called everytime a new client connects
#[spacetimedb::reducer(client_connected)]
#[shared_table_reducer]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    record_active_connection(ctx);

    if let Some(developer) = ctx.db.developer().identity().find(ctx.sender) {
        log::info!(
            "Developer identity connected for developer: {}, service: {}",
            developer.developer_name,
            developer.service_name
        );
        return Ok(());
    }

    if has_role(ctx, &ctx.sender, Role::SkipQueue) {
        return Ok(());
    }

    if ctx.db.blocked_identity().identity().find(ctx.sender).is_some() || !is_authenticated(ctx, &ctx.sender) {
        log::info!("Blocking identity {}", ctx.sender.to_hex());
        return Err("Unauthorized".into());
    }

    if ctx.db.user_state().identity().find(ctx.sender).is_some() {
        // Reconnecting while signed in keeps the session; sign_in is idempotent.
        return Ok(());
    }

    Err("Identity with no user or permission is disallowed from connecting".into())
}

// Newest connection wins. Updated on every connect so a stale connection's
// disconnect can be told apart from the active one's.
fn record_active_connection(ctx: &ReducerContext) {
    if let (Some(user), Some(connection_id)) = (ctx.db.user_state().identity().find(ctx.sender), ctx.connection_id) {
        let active = ActiveConnectionState {
            entity_id: user.entity_id,
            connection_id,
        };
        if ctx.db.active_connection_state().entity_id().find(user.entity_id).is_some() {
            ctx.db.active_connection_state().entity_id().update(active);
        } else {
            ctx.db.active_connection_state().insert(active);
        }
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user_state().identity().find(ctx.sender) {
        if let Some(active) = ctx.db.active_connection_state().entity_id().find(user.entity_id) {
            // Stale: the identity already opened a newer connection, i.e. the player
            // reconnected before this disconnect was processed. Signing out now would
            // end the live session.
            if ctx.connection_id != Some(active.connection_id) {
                log::info!("(sign_out) Stale connection disconnected for {:?}", ctx.sender.to_hex());
                return;
            }
            ctx.db.active_connection_state().entity_id().delete(user.entity_id);
        }
    }
    sign_out(ctx);
}

#[spacetimedb::reducer]
pub fn init_region_info(
    ctx: &ReducerContext,
    world_width: i32,
    world_height: i32,
    region_index: u8,
    region_count: u8,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if region_index > region_count {
        return Err("region_index cannot be greater than region_count".into());
    }
    let region_count_sqrt = (region_count as f32).sqrt() as u8;
    if region_count_sqrt * region_count_sqrt != region_count {
        return Err("region_count must be a square number".into());
    }

    let region_coord = RegionCoordinates::from_region_index(region_index, region_count_sqrt);
    if let Err(error) = ctx.db.world_region_state().try_insert(WorldRegionState {
        id: 0,
        region_width_chunks: world_width as u16,
        region_height_chunks: world_height as u16,
        region_min_chunk_x: (region_coord.x as i32 * world_width) as u16,
        region_min_chunk_z: (region_coord.z as i32 * world_height) as u16,
        region_index: region_index,
        region_count,
        region_count_sqrt,
    }) {
        log::error!("Failed to insert world region: {}", error);
        return Err(format!("Failed to insert world region: {}", error));
    }
    if let Err(error) = ctx.db.world_region_name_state().try_insert(WorldRegionNameState {
        id: 0,
        player_facing_name: format!("Region {}", region_index).into(),
        module_name_prefix: "bitcraft_region_".into(),
    }) {
        log::error!("Failed to insert world region name: {}", error);
        return Err(format!("Failed to insert world region name: {}", error));
    }

    let mut globals = ctx.db.globals().version().find(&0).unwrap();
    globals.region_index = region_index;
    ctx.db.globals().version().update(globals);

    Ok(())
}

#[spacetimedb::reducer]
pub fn start_generating_world(
    ctx: &ReducerContext,
    world_width: i32,
    world_height: i32,
    region_index: u8,
    region_count: u8,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let region_state = ctx.db.world_region_state().id().find(0).expect("world_region_state not found");
    if region_index != region_state.region_index
        || region_count != region_state.region_count
        || world_width != region_state.region_width_chunks as i32
        || world_height != region_state.region_height_chunks as i32
    {
        return Err("Arguments don't match world_region_state".into());
    }

    if let Err(error) = ctx.db.dimension_description_state().try_insert(DimensionDescriptionState {
        entity_id: 1,
        dimension_id: 1,
        dimension_type: messages::game_util::DimensionType::Overworld,
        interior_instance_id: 0,
        dimension_position_large_x: region_state.region_min_chunk_x as u32,
        dimension_position_large_z: region_state.region_min_chunk_z as u32,
        dimension_size_large_x: world_width as u32,
        dimension_size_large_z: world_height as u32,
        dimension_network_entity_id: 0,
        collapse_timestamp: 0,
    }) {
        log::error!("Failed to insert dimension description: {}", error);
        return Err(format!("Failed to insert dimension description: {}", error));
    }

    log::info!("Receiving world upload ({world_width}x{world_height} chunks)");

    Ok(())
}

pub fn world_loaded(ctx: &ReducerContext) -> bool {
    // if there is a resources log then we already loaded a world
    ctx.db.resources_log().version().find(0).is_some()
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn insert_terrain_chunk(
    ctx: &ReducerContext,
    terrain_chunk: TerrainChunkState,
    buildings: Vec<WorldGenGeneratedBuilding>,
    resources: Vec<WorldGenGeneratedResourceDeposit>,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if world_loaded(ctx) {
        return Ok(());
    }

    let span = DontCheckAreaAroundClaimSpan::start();
    insert_terrain_chunk(ctx, terrain_chunk);
    insert_buildings(ctx, buildings, false);
    insert_resources(ctx, resources);
    span.end();

    return Ok(());

    fn insert_terrain_chunk(ctx: &ReducerContext, terrain_chunk: TerrainChunkState) {
        let chunk_index = terrain_chunk.chunk_index;
        let x = terrain_chunk.chunk_x;
        let z = terrain_chunk.chunk_z;
        let d = terrain_chunk.dimension;

        if ctx.db.terrain_chunk_state().try_insert(terrain_chunk).is_err() {
            // This never gets called
            log::error!("Failed to insert terrain chunk with index {}", chunk_index,);
            return;
        }

        log::info!("Inserted chunk with index {chunk_index} (X: {x}, Z: {z}, D: {d})");
    }
}

fn insert_buildings(ctx: &ReducerContext, buildings: Vec<WorldGenGeneratedBuilding>, ignore_claim_creation: bool) {
    let mut building_count = 0;

    for building in buildings.iter() {
        let building_component = building.building.as_ref().unwrap();
        let building_description = ctx.db.building_desc().id().find(&building_component.building_description_id);
        if building_description.is_none() {
            continue;
        }

        let building_description = building_description.as_ref().unwrap();

        let claim_entity_id = building_component.claim_entity_id;

        let offset = OffsetCoordinatesSmall {
            x: building.x,
            z: building.z,
            dimension: building.dimension,
        };

        let entity_id = create_entity(ctx);
        insert_location(ctx, entity_id, offset);
        create_building_component(
            ctx,
            claim_entity_id,
            entity_id,
            building_component.direction_index,
            building_description,
            0,
        );

        create_building_footprint(ctx, entity_id, building_component.direction_index, building_description, &None);

        if !ignore_claim_creation {
            if let Err(error) = create_building_claim(ctx, entity_id, true) {
                log::error!("Failed to create building claim: {}", error);
                continue;
            }
        }

        if ctx
            .db
            .interior_network_desc()
            .building_id()
            .find(&building_description.id)
            .is_some()
        {
            if let Err(error) = create_building_interior(ctx, entity_id) {
                log::error!("Failed to create building interior: {}", error);
                continue;
            }
        }

        create_building_spawns(ctx, entity_id);

        building_count += 1;
    }

    log::info!("Inserted {} buildings", building_count);
}

fn insert_resources(ctx: &ReducerContext, resources: Vec<WorldGenGeneratedResourceDeposit>) {
    // Keep a counter of total resources inserted, to put in logs.
    let mut resource_count = 0;

    // Keep counters of the number of each resource type inserted,
    // to store in the `ResourceCount` table.
    // Keep the counters in-memory and do one `ResourceCount::insert` at the end,
    // rather than eagerly updating the counters in the database by `ResourceState::insert_one`.
    let mut resource_counters: std::collections::HashMap<i32, i32> = ctx
        .db
        .resource_count()
        .iter()
        .map(|counter| (counter.resource_id, counter.num_in_world))
        .collect();

    for resource in resources {
        let entity_id = create_entity(ctx);
        let resource_deposit = resource.deposit.as_ref().unwrap();

        let offset = OffsetCoordinatesSmall {
            x: resource.x,
            z: resource.z,
            dimension: resource.dimension,
        };

        let deposit_description = ctx.db.resource_desc().id().find(&resource_deposit.resource_id).unwrap();
        let _ = ResourceState::spawn(
            ctx,
            Some(entity_id),
            resource_deposit.resource_id,
            offset.into(),
            resource_deposit.direction_index,
            deposit_description.max_health,
            false,
            false,
        );

        *resource_counters.entry(resource_deposit.resource_id).or_insert(0) += 1;

        resource_count += 1;
    }

    log::info!("Inserted {} resources", resource_count);

    // Store the count of deposits of each type of resource in the database,
    // so the resource regen loop knows how many more to spawn at each iteration.
    for (resource_id, num_in_world) in resource_counters.into_iter() {
        if let Some(mut resource_count) = ctx.db.resource_count().resource_id().find(resource_id) {
            resource_count.num_in_world = num_in_world;
            ctx.db.resource_count().resource_id().update(resource_count);
        } else {
            ctx.db.resource_count().insert(ResourceCount { resource_id, num_in_world });
        }
    }
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn insert_resources_log(ctx: &ReducerContext, resources_log: ResourcesLog) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if world_loaded(ctx) {
        log::info!("Skipped world generation due to World already uploaded.");
        return Ok(());
    }

    let world_width = resources_log.world_width as i32;
    let world_height = resources_log.world_height as i32;
    let total_chunks = world_width * world_height;

    if ctx.db.terrain_chunk_state().dimension().filter(dimensions::OVERWORLD).count() as i32 != total_chunks {
        log::error!(
            "Failed to insert resources log: amount of TerrainChunks does not match the given world-size. Expected: {}, got: {}",
            total_chunks,
            ctx.db.terrain_chunk_state().count(),
        );
        return Err("Failed to insert resources log: amount of TerrainChunks does not match the given world-size".into());
    }

    match ctx.db.resources_log().try_insert(resources_log) {
        Ok(_) => {
            log::info!("Inserted resources log")
        }
        Err(error) => {
            log::error!("Failed to insert resources log: {}", error);
            return Err(format!("Failed to insert resources log: {}", error));
        }
    }

    //If amount of chunks is equal to total chunks
    LocationCache::build(ctx);

    ResourcesLog::populate_single_resource_chunks_info(ctx);

    //Re-enable agents in case they were disabled after deleting a world
    let mut config = ctx.db.config().version().find(&0).unwrap();
    config.agents_enabled = true;
    ctx.db.config().version().update(config);

    let cur_region = ctx.db.globals().version().find(0).expect("Missing globals").region_index;
    let is_dev = ctx.db.config().version().find(0).expect("Missing config").env == "dev";
    if ctx.db.region_control_info().region_id().find(cur_region).is_none() {
        RegionControlInfo::insert_shared(
            ctx,
            RegionControlInfo {
                region_id: cur_region,
                initialized: true,
                allow_players: is_dev,
                allow_player_spawns: is_dev,
            },
            inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
        );
    }

    agents::init(ctx);

    log::info!("World generated");

    return Ok(());
}

#[spacetimedb::reducer]
pub fn generate_world(ctx: &ReducerContext, world_definition: WorldGenWorldDefinition) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if world_loaded(ctx) {
        log::info!("generate_world failed: World already loaded");
        return Err("generate_world failed: World already loaded".into());
    }

    let mut world_definition = WorldDefinition::new_proto(world_definition);
    let generated_graph = WorldGraph::new(ctx, &mut world_definition);
    let generated_world = world_generator::generate(&world_definition, &generated_graph);

    commit_generated_world(ctx, generated_world);

    ResourcesLog::save(ctx, &generated_graph, &world_definition);
    ResourcesLog::populate_single_resource_chunks_info(ctx);

    Ok(())
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn generate_dev_island(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if world_loaded(ctx) {
        log::info!("generate_dev_island failed: World already loaded");
        return Err("generate_dev_island failed: World already loaded".into());
    }

    if let Some(generated_world) = dev_island::generate(ctx) {
        commit_generated_world(ctx, generated_world);
        return Ok(());
    };

    Err("Failed to generate world".into())
}

#[spacetimedb::reducer]
pub fn generate_flat_world(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }

    if world_loaded(ctx) {
        log::info!("generate_flat_world failed: World already loaded");
        return Err("generate_flat_world failed: World already loaded".into());
    }

    let generated_world = flat_world::generate();

    commit_generated_world(ctx, generated_world);

    Ok(())
}

fn commit_generated_world(ctx: &ReducerContext, generated_world: GeneratedWorld) {
    for dimension in generated_world.dimensions {
        if ctx.db.dimension_description_state().try_insert(dimension.clone()).is_err() {
            log::error!("Failed to insert dimension description");
        }
    }

    log::info!(
        "Inserting chunks w:{} h:{}",
        generated_world.chunks.len(),
        generated_world.chunks[0].len()
    );

    for j in 0..generated_world.chunks.len() {
        for i in 0..generated_world.chunks[j].len() {
            if ctx
                .db
                .terrain_chunk_state()
                .try_insert(generated_world.chunks[i][j].clone())
                .is_err()
            {
                log::error!("Failed to insert terrain chunk");
            }
        }
    }

    log::info!("Inserting buildings (this takes a while because of claims now)...");

    insert_buildings(ctx, generated_world.buildings, generated_world.ignore_claim_creation);
    insert_resources(ctx, generated_world.deposits);

    // Keep a counter of total inserted, to put in logs.
    let mut enemy_count = 0;

    for enemy in generated_world.enemies.iter() {
        //note: no herds exist

        // let herd_source = HerdState::iter().nth(0);
        // if let Some(herd_source) = herd_source {
        //     let mut herd = herd_source.clone();

        let enemy_state = EnemyState::new(ctx, enemy.enemy.as_ref().unwrap().enemy_type, 0); //herd.entity_id);

        let enemy_type = enemy_state.enemy_type as i32;
        let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

        let _ = EnemyState::spawn_enemy(
            ctx,
            &enemy_desc,
            enemy_state,
            OffsetCoordinatesSmall {
                x: enemy.x,
                z: enemy.z,
                dimension: enemy.dimension,
            },
            None,
        );

        // herd.current_population += 1;
        // herd.ignore_eagerness = false; // no longer need to spawn everything at once

        // let herd_entity_id = herd.entity_id;
        // ctx.db.herd_state().entity_id().update(herd);
        // }
        enemy_count += 1;
    }

    log::info!("Enemy count : {}", enemy_count);

    let mut dropped_inventory_count = 0;

    for dropped_inventory in generated_world.dropped_inventories.iter() {
        let entity_id = create_entity(ctx);
        let dropped_inventory_state = dropped_inventory.dropped_inventory.as_ref().unwrap();
        let mut comp = dropped_inventory_state.clone();
        comp.entity_id = entity_id;
        if ctx.db.dropped_inventory_state().try_insert(comp).is_err() {
            log::error!("Failed to insert dropped_inventory");
        }
        game_state::insert_location(
            ctx,
            entity_id,
            OffsetCoordinatesSmall {
                x: dropped_inventory.x,
                z: dropped_inventory.z,
                dimension: dropped_inventory.dimension,
            },
        );

        dropped_inventory_count += 1;
    }

    log::info!("Dropped Inventory count : {}", dropped_inventory_count);

    let mut npc_count = 0;

    for npc in generated_world.npcs.iter() {
        NpcState::spawn(
            ctx,
            npc.npc.as_ref().unwrap().npc_type,
            0,
            0,
            OffsetCoordinatesSmall {
                x: npc.x,
                z: npc.z,
                dimension: npc.dimension,
            },
            false,
        );

        npc_count += 1;
    }

    log::info!("NPC count : {}", npc_count);

    LocationCache::build(ctx);

    //note: no insert if from dev world. Also called before save in other methods
    // ResourcesLog::populate_single_resource_chunks_info();

    //Re-enable agents in case they were disabled after deleting a world
    let mut config = ctx.db.config().version().find(&0).unwrap();
    config.agents_enabled = true;
    ctx.db.config().version().update(config);

    agents::init(ctx);

    log::info!("World generated");
}

#[spacetimedb::reducer]
pub fn stop_agents(ctx: &ReducerContext) {
    //DAB Note: rename this to pause_agents
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return;
    }

    let mut config = ctx.db.config().version().find(&0).unwrap().clone();
    if !config.agents_enabled {
        return;
    }

    config.agents_enabled = false;
    ctx.db.config().version().update(config);

    log::info!("Agents stopped");
}

#[spacetimedb::reducer]
pub fn start_agents(ctx: &ReducerContext) {
    //DAB Note: rename this to resume_agents
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return;
    }

    let mut config = ctx.db.config().version().find(&0).unwrap().clone();
    if config.agents_enabled {
        return;
    }

    config.agents_enabled = true;
    ctx.db.config().version().update(config);

    //DO NOT re-init agents - stopping them will leave them running, but not doing anything
    //if TerrainChunkState::iter().next().is_some() {
    //    agents::init();
    //}

    log::info!("Agents resumed");
}

#[spacetimedb::reducer]
pub fn force_start_agents(ctx: &ReducerContext) {
    // !!! WARNING - THIS MAY CAUSE AGENTS TO GET DUPLICATED !!!
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return;
    }

    let mut config = ctx.db.config().version().find(&0).unwrap().clone();
    if config.agents_enabled {
        return;
    }

    config.agents_enabled = true;
    ctx.db.config().version().update(config);

    // only start it back up if there is a world loaded
    if ctx.db.terrain_chunk_state().iter().next().is_some() {
        agents::init(ctx);
    }

    log::info!("Agents force-restarted");
}
