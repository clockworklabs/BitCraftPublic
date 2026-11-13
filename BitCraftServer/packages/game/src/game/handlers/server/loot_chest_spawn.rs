use spacetimedb::ReducerContext;

use crate::game::coordinates::*;
use crate::game::game_state::{self, game_state_filters};
use crate::game::reducer_helpers::loot_chest_helpers;
use crate::messages::authentication::ServerIdentity;
use crate::{building_spawn_desc, building_state};

#[spacetimedb::table(accessor = loot_chest_spawn_timer, scheduled(loot_chest_spawn, at = scheduled_at))]
pub struct LootChestSpawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub building_spawn_id: i32,
    pub building_entity_id: u64,
}

#[spacetimedb::reducer]
pub fn loot_chest_spawn(ctx: &ReducerContext, timer: LootChestSpawnTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    reduce(ctx, timer.building_entity_id, timer.building_spawn_id)
}

pub fn reduce(ctx: &ReducerContext, building_entity_id: u64, building_spawn_id: i32) -> Result<(), String> {
    // Designers: Change this to true to see relling output for ancient ruins loot chests
    let verbose = false;

    let building_spawn_desc = ctx.db.building_spawn_desc().id().find(building_spawn_id).unwrap();

    let building = ctx.db.building_state().entity_id().find(building_entity_id).unwrap();
    let building_coordinates = game_state_filters::coordinates(ctx, building_entity_id);

    let spawn_location = building_spawn_desc.get_spawn_coordinates(&building_coordinates, building.direction_index);
    let direction_index = (HexDirection::from(building_spawn_desc.direction) + HexDirection::from(building.direction_index)) as i32;

    let entity_id = game_state::create_entity(ctx);
    loot_chest_helpers::spawn_loot_chest(
        ctx,
        &building_spawn_desc.spawn_ids,
        entity_id,
        spawn_location,
        direction_index,
        building_entity_id,
        building_spawn_id,
        verbose,
    )
}
