use super::world_generator::GeneratedWorld;
use crate::game::coordinates::hex_coordinates::HexCoordinates;
use crate::game::coordinates::offset_coordinates::OffsetCoordinates;
use crate::game::coordinates::{LargeHexTile, OffsetCoordinatesLarge, OffsetCoordinatesSmall, SmallHexTile};
use crate::messages::components::{DimensionDescriptionState, EnemyState, EnemyStatus, InventoryState, NpcState};
use crate::messages::components::{DroppedInventoryState, TerrainChunkState};
use crate::messages::game_util::{ItemStack, Pocket};
use crate::messages::static_data::ResourceDesc;
use crate::messages::static_data::{BuildingDesc, CargoDesc, EnemyDesc, FootprintTile, NpcDesc};
use crate::{
    building_desc, cargo_desc, enemy_desc, extraction_recipe_desc, npc_desc, resource_desc, start_generating_world, EnemyType, NpcType,
};
use crate::{
    messages::components::{BuildingState, ResourceState, TerrainCell},
    messages::game_util::DimensionType,
    messages::world_gen::*,
};
use spacetimedb::{log, ReducerContext, Table, Timestamp};
use std::collections::HashMap;

const WATER_LEVEL: i16 = 20;
const ZONING_TYPE_PLAYER_START_CELL: u8 = 1;
const MAX_SAFETY_COUNT: i32 = 10000;
const CHUNK_INDEX_MULTIPLIER: i32 = 1000;

pub fn generate(ctx: &ReducerContext) -> Option<GeneratedWorld> {
    log::info!("Generating dev world.");

    let world_chunk_width: i32 = 10;
    let world_chunk_height: i32 = 10;

    if let Err(error) = start_generating_world(ctx, world_chunk_width, world_chunk_height, 1, 1) {
        log::error!("Failed to generate world: {}", error);
        return None;
    }

    let height = (TerrainChunkState::HEIGHT as i32) * world_chunk_height;
    let width = (TerrainChunkState::WIDTH as i32) * world_chunk_width;

    let center = HexCoordinates::from_offset_coordinates(height / 2, width / 2, 1);
    let mut generated_world = create_empty_world(world_chunk_width, world_chunk_height);
    initialize_chunks(&mut generated_world, world_chunk_width, world_chunk_height, center);
    place_elements(ctx, &mut generated_world, center, width / 2, height / 2);

    let width_half = width / 2;
    let height_half = height / 2;

    let area_gap = 3;

    let water_x = width_half - area_gap + 1;
    let water_z = height_half + area_gap - 1;

    generate_terrain(height, width, center, water_x, water_z, &mut generated_world);

    Some(generated_world)
}

fn create_empty_world(world_chunk_width: i32, world_chunk_height: i32) -> GeneratedWorld {
    GeneratedWorld {
        chunks: vec![],
        buildings: vec![],
        deposits: vec![],
        enemies: vec![],
        dropped_inventories: vec![],
        npcs: vec![],
        ignore_claim_creation: true,
        dimensions: vec![DimensionDescriptionState {
            entity_id: 1,
            dimension_id: 1,

            dimension_type: DimensionType::Overworld,
            interior_instance_id: 0,
            dimension_position_large_x: 0,
            dimension_position_large_z: 0,
            dimension_size_large_x: world_chunk_width as u32,
            dimension_size_large_z: world_chunk_height as u32,
            dimension_network_entity_id: 0,
            collapse_timestamp: 0,
        }],
    }
}

fn initialize_chunks(generated_world: &mut GeneratedWorld, width: i32, height: i32, _center: HexCoordinates) {
    for i in 0..width {
        generated_world.chunks.push(Vec::new());
        for j in 0..height {
            let mut chunk = TerrainChunkState::default_with_capacity();
            chunk.chunk_x = i;
            chunk.chunk_z = j;
            chunk.chunk_index = (j * CHUNK_INDEX_MULTIPLIER + i + 1) as u64;
            generated_world.chunks[i as usize].push(chunk);
        }
    }
}

fn place_elements(ctx: &ReducerContext, generated_world: &mut GeneratedWorld, center: HexCoordinates, width_half: i32, height_half: i32) {
    let mut occupied_positions: HashMap<SmallHexTile, i32> = HashMap::new();

    let step = 2;
    let area_gap = step;
    let x_start = width_half + area_gap;
    let z_start = height_half - area_gap;

    let count_in_row = 20;

    place_buildings(
        ctx,
        generated_world,
        center,
        x_start,
        z_start,
        step,
        -step,
        count_in_row,
        &mut occupied_positions,
    );

    //land resources
    let z_start = height_half + area_gap;
    place_resources(
        ctx,
        generated_world,
        center,
        x_start,
        z_start,
        step,
        step,
        count_in_row,
        &mut occupied_positions,
        true,
    );

    //water resources
    let x_start = width_half - area_gap;
    place_resources(
        ctx,
        generated_world,
        center,
        x_start,
        z_start,
        -step,
        step,
        5,
        &mut occupied_positions,
        false,
    );

    let x_start = width_half - area_gap;
    let z_start = height_half - area_gap;

    let dropped_inventory_step = 1;
    let dropped_inventory_count_in_row = 10;
    place_dropped_inventories(
        ctx,
        generated_world,
        center,
        x_start,
        z_start,
        -dropped_inventory_step,
        -dropped_inventory_step,
        dropped_inventory_count_in_row,
        &mut occupied_positions,
    );
    place_enemies(
        ctx,
        generated_world,
        center,
        x_start - dropped_inventory_count_in_row - area_gap,
        z_start,
        -step,
        -step,
        area_gap,
        &mut occupied_positions,
    );
    place_npcs(
        ctx,
        generated_world,
        center,
        x_start - dropped_inventory_count_in_row - (area_gap * 2),
        z_start,
        -step,
        -step,
        area_gap,
        &mut occupied_positions,
    );
}

fn generate_terrain(height: i32, width: i32, center: HexCoordinates, water_x: i32, water_z: i32, generated_world: &mut GeneratedWorld) {
    let island_radius = 128i16;
    // let biomes_count = (Biome::iter().count() - 1) as i16;
    // let biome_slice = island_radius / biomes_count;

    for x in 0..height {
        for z in 0..width {
            let offset = OffsetCoordinates { x, z, dimension: 1 };

            let coord = HexCoordinates::from_offset_coordinates(x, z, 1);
            let small_offset_coords = OffsetCoordinatesSmall::from(offset);

            // let mut elevation = 10;
            let d = coord.distance_to(center) as i16;
            let mut elevation = WATER_LEVEL;

            if d > island_radius {
                elevation = WATER_LEVEL + (island_radius - d);
            } else if x < water_x && z > water_z {
                elevation = WATER_LEVEL - 10;
            }

            let biomes = 0; //((d / biome_slice) + 1)  as u32; // Dev Biome
                            // biomes = biomes.max((biomes_count) as u32);
            let biome_density = 30;

            // if elevation > 22 && elevation < 28 {
            //     biomes = 7; // Desert for low cliffs
            // } else if elevation > 28 {
            //     biomes = 3; // Snow for cliffs
            // }

            let zoning_type = if d < 2 { ZONING_TYPE_PLAYER_START_CELL } else { 0 };

            let terrain_cell = TerrainCell {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                elevation,
                water_level: WATER_LEVEL,
                zoning_type,
                biomes,
                original_elevation: elevation,
                biome_density,
                ..Default::default()
            };

            let chunk_indices = (
                offset.x / TerrainChunkState::WIDTH as i32,
                offset.z / TerrainChunkState::HEIGHT as i32,
            );

            let chunk = &mut generated_world.chunks[chunk_indices.0 as usize][chunk_indices.1 as usize];
            chunk.set_entity(offset.into(), terrain_cell);
        }
    }
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_buildings(
    ctx: &ReducerContext,
    generated_world: &mut GeneratedWorld,
    center: HexCoordinates,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    x_count_max: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
) {
    let mut buildings: Vec<_> = ctx.db.building_desc().iter().collect();
    buildings.sort_by_key(|building| building.footprint.len()); // Sort by footprint size

    log::info!("Placing {} buildings...", buildings.len());

    let ignored_buildings: Vec<i32> = vec![
        109263244,  //LargePyreliteCaveInterior
        1295620670, //SmallPyreliteCaveInterior
    ];

    place_entities(
        generated_world,
        &buildings,
        |world, building, small_hex_tile, positions| {
            if ignored_buildings.contains(&building.id) {
                return;
            }

            let small_offset_coords = OffsetCoordinatesSmall::from(small_hex_tile);

            // Add the building to the generated world
            world.buildings.push(WorldGenGeneratedBuilding {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                building: Some(BuildingState {
                    entity_id: 0,
                    claim_entity_id: 0,
                    direction_index: 0,
                    building_description_id: building.id,
                    constructed_by_player_entity_id: 0,
                }),
                dimension: 1,
            });

            //add footprint
            for footprint_tile in &building.footprint {
                let footprint_tile_world = SmallHexTile {
                    x: small_hex_tile.x + footprint_tile.x,
                    z: small_hex_tile.z + footprint_tile.z,
                    dimension: center.dimension,
                };
                positions.entry(footprint_tile_world).or_insert(1);
            }

            log::info!(
                "Successfully placed building {} at ({},{})",
                building.name,
                small_offset_coords.x,
                small_offset_coords.z
            );
        },
        can_place_building, // Custom function to validate building placement
        x_start,
        z_start,
        step_x,
        step_z,
        x_count_max,
        occupied_positions,
    );
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_resources(
    ctx: &ReducerContext,
    generated_world: &mut GeneratedWorld,
    center: HexCoordinates,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    x_count_max: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
    land_only: bool,
) {
    let mut resources: Vec<_> = ctx
        .db
        .resource_desc()
        .iter()
        .filter(|res| {
            //note: hacky, fishing rod requirement for water-only resources
            let mut recipes = ctx.db.extraction_recipe_desc().resource_id().filter(res.id);
            if recipes.any(|a| a.tool_requirements.iter().any(|b| b.tool_type == 10)) {
                !land_only
            } else {
                land_only
            }
        })
        .collect();

    resources.sort_by_key(|resource| resource.footprint.len());

    log::info!("Placing {} resources . . .", resources.len());

    place_entities(
        generated_world,
        &resources,
        |world, resource, small_hex_tile, positions| {
            let small_offset_coords = OffsetCoordinatesSmall::from(small_hex_tile);
            world.deposits.push(WorldGenGeneratedResourceDeposit {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                deposit: Some(ResourceState {
                    entity_id: 0,
                    resource_id: resource.id,
                    direction_index: -1,
                }),
                dimension: 1,
            });

            //add footprint
            for footprint_tile in &resource.footprint {
                let footprint_tile_world = SmallHexTile {
                    x: small_hex_tile.x + footprint_tile.x,
                    z: small_hex_tile.z + footprint_tile.z,
                    dimension: center.dimension,
                };
                positions.entry(footprint_tile_world).or_insert(1);
            }

            log::info!(
                "Successfully added resource {} at ({},{})",
                resource.name,
                small_offset_coords.x,
                small_offset_coords.z
            );
        },
        can_place_resource,
        x_start,
        z_start,
        step_x,
        step_z,
        x_count_max,
        occupied_positions,
    );
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_dropped_inventories(
    ctx: &ReducerContext,
    generated_world: &mut GeneratedWorld,
    _center: HexCoordinates,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    x_count_max: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
) {
    let cargos: Vec<_> = ctx.db.cargo_desc().iter().collect();
    log::info!("Placing {} dropped inventories of each cargo . . .", cargos.len());

    place_entities(
        generated_world,
        &cargos,
        |world, cargo, small_hex_tile, positions| {
            let small_offset_coords = OffsetCoordinatesSmall::from(small_hex_tile);
            world.dropped_inventories.push(WorldGenGeneratedDroppedInventory {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                dropped_inventory: Some(DroppedInventoryState {
                    entity_id: 0,
                    owner_entity_id: 0,
                    active_timer_id: 0,
                }),
                inventory: Some(InventoryState {
                    entity_id: 0,
                    pockets: vec![
                        Pocket::empty(6000),
                        Pocket {
                            volume: 6000,
                            contents: Some(ItemStack::single_cargo(cargo.id)),
                            locked: false,
                        },
                    ],
                    inventory_index: 0,
                    cargo_index: 1,
                    owner_entity_id: 0,
                    player_owner_entity_id: 0,
                }),
                dimension: 1,
            });

            positions.entry(small_hex_tile).or_insert(1);
            log::info!(
                "Successfully added dropped_inventory {} at ({},{})",
                cargo.name,
                small_offset_coords.x,
                small_offset_coords.z
            );
        },
        can_place_cargo,
        x_start,
        z_start,
        step_x,
        step_z,
        x_count_max,
        occupied_positions,
    );
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_npcs(
    ctx: &ReducerContext,
    generated_world: &mut GeneratedWorld,
    _center: HexCoordinates,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    x_count_max: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
) {
    let npcs: Vec<_> = ctx.db.npc_desc().iter().collect();
    log::info!("Placing {} NPCs . . .", npcs.len());
    place_entities(
        generated_world,
        &npcs,
        |world, npc, small_hex_tile, positions| {
            let small_offset_coords = OffsetCoordinatesSmall::from(small_hex_tile);
            let npc_type = NpcType::to_enum(npc.npc_type);

            world.npcs.push(WorldGenGeneratedNPC {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                npc: Some(NpcState {
                    entity_id: 0,
                    npc_type,
                    direction: 0,
                    building_entity_id: 0,
                    next_action_timestamp: NpcState::get_next_timestamp(ctx, npc_type),
                    move_duration: 0.0,
                    started_moving: 0,
                    previous_buildings: vec![],
                    traveling: false,
                }),
                dimension: 1,
            });

            positions.entry(small_hex_tile).or_insert(1);

            log::info!(
                "Successfully added NPC {} at ({},{})",
                npc.name,
                small_offset_coords.x,
                small_offset_coords.z
            );
        },
        can_place_npc,
        x_start,
        z_start,
        step_x,
        step_z,
        x_count_max,
        occupied_positions,
    );
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_enemies(
    ctx: &ReducerContext,
    generated_world: &mut GeneratedWorld,
    _center: HexCoordinates,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    x_count_max: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
) {
    let enemies: Vec<_> = ctx.db.enemy_desc().iter().collect();
    log::info!("Placing {} enemies . . .", enemies.len());

    place_entities(
        generated_world,
        &enemies,
        |world, enemy, small_hex_tile, positions| {
            let small_offset_coords = OffsetCoordinatesSmall::from(small_hex_tile);
            let enemy_type = EnemyType::to_enum(enemy.enemy_type);
            world.enemies.push(WorldGenGeneratedEnemy {
                x: small_offset_coords.x,
                z: small_offset_coords.z,
                enemy: Some(EnemyState {
                    entity_id: 0,
                    direction: -1,
                    status: EnemyStatus::Inactive,
                    herd_entity_id: 0,
                    last_ranged_attack_timestamp: Timestamp::UNIX_EPOCH,
                    enemy_type,
                }),
                dimension: 1,
            });

            positions.entry(small_hex_tile).or_insert(1);
            log::info!(
                "Successfully added enemy {} at ({},{})",
                enemy.name,
                small_offset_coords.x,
                small_offset_coords.z
            );
        },
        can_place_enemy,
        x_start,
        z_start,
        step_x,
        step_z,
        x_count_max,
        occupied_positions,
    );
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
fn place_entities<T>(
    generated_world: &mut GeneratedWorld,
    entities: &[T],
    add_fn: impl Fn(&mut GeneratedWorld, &T, SmallHexTile, &mut HashMap<SmallHexTile, i32>),
    can_place_fn: impl Fn(SmallHexTile, &T, &HashMap<SmallHexTile, i32>) -> bool,
    x_start: i32,
    z_start: i32,
    step_x: i32,
    step_z: i32,
    max_x_count: i32,
    occupied_positions: &mut HashMap<SmallHexTile, i32>,
) {
    let mut next_x = x_start;
    let mut next_z = z_start;

    let mut x_count = 0;
    let mut safety_count = 0;

    for entity in entities {
        loop {
            if safety_count > MAX_SAFETY_COUNT {
                break;
            }
            safety_count += 1;

            let offset = OffsetCoordinatesLarge {
                x: next_x,
                z: next_z,
                dimension: 1,
            };
            let small_hex_tile = LargeHexTile::from(offset).center_small_tile();

            if can_place_fn(small_hex_tile, entity, occupied_positions) {
                add_fn(generated_world, entity, small_hex_tile, occupied_positions);
                break;
            }

            next_x += step_x;
            x_count += 1;

            if x_count >= max_x_count {
                x_count = 0;
                next_x = x_start;
                next_z += step_z;
            }
        }
    }
}

fn can_place_npc(center: SmallHexTile, _npc: &NpcDesc, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    //validate position
    !occupied_positions.contains_key(&center)
}

fn can_place_enemy(center: SmallHexTile, _enemy: &EnemyDesc, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    //validate position
    !occupied_positions.contains_key(&center)
}

fn can_place_cargo(center: SmallHexTile, _cargo: &CargoDesc, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    //validate position
    !occupied_positions.contains_key(&center)
}

fn can_place_resource(center: SmallHexTile, resource: &ResourceDesc, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    //validate footprint
    valid_footprint(center, &resource.footprint, occupied_positions)
}

fn can_place_building(center: SmallHexTile, building: &BuildingDesc, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    //validate footprint
    valid_footprint(center, &building.footprint, occupied_positions)
}

fn valid_footprint(center: SmallHexTile, footprint: &Vec<FootprintTile>, occupied_positions: &HashMap<SmallHexTile, i32>) -> bool {
    for footprint_tile in footprint {
        let footprint_tile_world = SmallHexTile {
            x: center.x + footprint_tile.x,
            z: center.z + footprint_tile.z,
            dimension: center.dimension,
        };
        if occupied_positions.contains_key(&footprint_tile_world) {
            return false;
        }
    }

    true
}
