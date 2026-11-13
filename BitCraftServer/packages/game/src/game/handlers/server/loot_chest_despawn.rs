use spacetimedb::rand::Rng;
use spacetimedb::{ReducerContext, Table};

use crate::game::handlers::server::loot_chest_spawn::{loot_chest_spawn_timer, LootChestSpawnTimer};
use crate::game::reducer_helpers::footprint_helpers;
use crate::game::reducer_helpers::timer_helpers::now_plus_secs_f32;
use crate::{building_spawn_desc, inventory_state, location_state, loot_chest_state};
use crate::{messages::authentication::ServerIdentity, unwrap_or_err};

#[spacetimedb::table(accessor = loot_chest_despawn_timer, scheduled(loot_chest_despawn, at = scheduled_at))]
pub struct LootChestDespawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub loot_chest_entity_id: u64,
    pub should_respawn: bool,
}

#[spacetimedb::reducer]
pub fn loot_chest_despawn(ctx: &ReducerContext, timer: LootChestDespawnTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let loot_chest_entity_id = timer.loot_chest_entity_id;
    unwrap_or_err!(
        ctx.db.loot_chest_state().entity_id().find(loot_chest_entity_id),
        "Invalid LootChest"
    );

    footprint_helpers::delete_footprint(ctx, loot_chest_entity_id);

    if timer.should_respawn {
        let loot_chest = ctx.db.loot_chest_state().entity_id().find(loot_chest_entity_id).unwrap();
        let building_entity_id = loot_chest.building_entity_id;
        let building_spawn_id = loot_chest.building_spawn_id;

        let building_spawn = ctx.db.building_spawn_desc().id().find(loot_chest.building_spawn_id).unwrap();
        let delay = ctx
            .rng()
            .gen_range(building_spawn.respawn_time_min..building_spawn.respawn_time_max);

        ctx.db
            .loot_chest_spawn_timer()
            .try_insert(LootChestSpawnTimer {
                scheduled_id: 0,
                scheduled_at: now_plus_secs_f32(delay, ctx.timestamp),
                building_entity_id,
                building_spawn_id,
            })
            .ok()
            .unwrap();
    }

    ctx.db.location_state().entity_id().delete(loot_chest_entity_id);
    ctx.db.inventory_state().entity_id().delete(loot_chest_entity_id);
    ctx.db.loot_chest_state().entity_id().delete(loot_chest_entity_id);

    Ok(())
}
