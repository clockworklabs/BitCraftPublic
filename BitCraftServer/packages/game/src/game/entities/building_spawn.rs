use spacetimedb::{log, ReducerContext, Table};

pub use crate::game::coordinates::*;
use crate::{
    building_state,
    game::{
        game_state::{self, game_state_filters},
        handlers::server::loot_chest_spawn,
        reducer_helpers::building_helpers::create_building_unsafe,
    },
    herd_state,
    messages::{
        components::{attached_herds_state, AttachedHerdsState, EnemyState, HerdState},
        static_data::*,
    },
    unwrap_or_err, NpcState, PavedTileState, ResourceState,
};

impl BuildingSpawnDesc {
    pub fn get_spawn_coordinates(&self, building_coordinates: &SmallHexTile, building_direction_index: i32) -> SmallHexTile {
        SmallHexTile {
            x: building_coordinates.x + self.x,
            z: building_coordinates.z + self.z,
            dimension: building_coordinates.dimension,
        }
        .rotate_around(building_coordinates, building_direction_index / 2)
    }

    pub fn get_traveler_spawn_coordinates(
        ctx: &ReducerContext,
        building_desc_id: i32,
        building_coordinates: &SmallHexTile,
        building_direction_index: i32,
    ) -> SmallHexTile {
        let mut building_spawns = ctx.db.building_spawn_desc().building_id().filter(building_desc_id);

        let traveler_spawn = building_spawns.find(|bs| bs.spawn_type == BuildingSpawnType::TravelerCamp)
            .unwrap();

        traveler_spawn.get_spawn_coordinates(building_coordinates, building_direction_index)
    }

    pub fn get_traveler_direction(ctx: &ReducerContext, building_desc_id: i32, building_direction_index: i32) -> i32 {
        let mut building_spawns = ctx.db.building_spawn_desc().building_id().filter(building_desc_id);

        if let Some(traveler_spawn) = building_spawns.find(|bs| bs.spawn_type == BuildingSpawnType::TravelerCamp) {
            return traveler_spawn.direction + building_direction_index;
        }
        building_direction_index
    }

    pub fn spawn_all(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
        let building = unwrap_or_err!(
            ctx.db.building_state().entity_id().find(building_entity_id),
            "Source Building does not exist"
        );
        let building_coords = game_state_filters::coordinates(ctx, building_entity_id);
        let building_direction = HexDirection::from(building.direction_index);

        for building_spawn in ctx.db.building_spawn_desc().building_id().filter(building.building_description_id) {
            let building_spawn_direction = (HexDirection::from(building_spawn.direction) + building_direction) as i32;
            match building_spawn.spawn_type {
                BuildingSpawnType::Chest => {
                    loot_chest_spawn::reduce(ctx, building.entity_id, building_spawn.id)?;
                }
                BuildingSpawnType::Building => {
                    let coord = building_spawn.get_spawn_coordinates(&building_coords, building.direction_index);
                    create_building_unsafe(ctx, 0, None, coord, building_spawn_direction, building_spawn.spawn_ids[0], None)?;
                }
                BuildingSpawnType::Paving => {
                    let coord = building_spawn.get_spawn_coordinates(&building_coords, building.direction_index);
                    PavedTileState::create_paving_unsafe(ctx, coord, building_spawn.spawn_ids[0], building_entity_id)?;
                }
                BuildingSpawnType::Resource => {
                    let coord = building_spawn.get_spawn_coordinates(&building_coords, building.direction_index);
                    let resource_id = building_spawn.spawn_ids[0];
                    let health = ctx.db.resource_desc().id().find(resource_id).unwrap().max_health;
                    // Note: this might be hurting slightly the eco-system; should these resources be taken into account by resource_regen? Right now they are.
                    // Adding support so they aren't will require an extra field to the deposit.
                    ResourceState::spawn(ctx, None, resource_id, coord, building_spawn_direction, health, false, false)?;
                }
                BuildingSpawnType::TravelerCamp => {}
                BuildingSpawnType::StationaryNpc => {
                    let coord = building_spawn.get_spawn_coordinates(&building_coords, building.direction_index);
                    NpcState::spawn(
                        ctx,
                        building_spawn.traveler_type.unwrap(),
                        building_spawn_direction,
                        building_entity_id,
                        coord.into(),
                        false,
                    );
                }
                BuildingSpawnType::Enemy => {
                    // create a new herd for that enemy entry. It will be used for respawn
                    // [MIGRATION TODO] We WILL want to add an EnemyAIParams id instead of an enemy type in BuildingSpawns.
                    let coord: crate::messages::util::SmallHexTileMessage =
                        building_spawn.get_spawn_coordinates(&building_coords, building.direction_index);
                    let mut herd = HerdState::new(ctx, 0); // [MIGRATION TODO] it would be nice to be able to set a herd state in interiors
                    herd.current_population = 1;
                    let herd_entity_id = herd.entity_id;
                    game_state::insert_location(ctx, herd_entity_id, coord.into());
                    let enemy_state = EnemyState::new(ctx, building_spawn.enemy_type.unwrap(), herd_entity_id);
                    let enemy_type = building_spawn.enemy_type.unwrap() as i32;
                    let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

                    ctx.db.attached_herds_state().try_insert(AttachedHerdsState {
                        entity_id: building_entity_id,
                        herds_entity_ids: vec![herd.entity_id],
                    })?;

                    match EnemyState::spawn_enemy(ctx, &enemy_desc, enemy_state, coord.into(), Some(&herd)) {
                        Ok(()) => {}
                        Err(s) => log::error!("{}", s),
                    };
                    let _ = ctx.db.herd_state().try_insert(herd);
                }
            }
        }
        Ok(())
    }
}
