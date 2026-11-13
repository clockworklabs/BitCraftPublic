use spacetimedb::{ReducerContext, Table};

use crate::{
    ThreatState, action_state, agents::crumb_trail_clean_up_agent, enemy_state, game::{autogen::_delete_entity::delete_entity, game_state::unix_ms, reducer_helpers::player_action_helpers::post_reducer_update_cargo}, herd_state, messages::{
        action_request::EnemySpawnLootRequest,
        authentication::ServerIdentity,
        components::{ContributionState, InventoryState, ability_state, crumb_trail_contribution_lock_state, crumb_trail_state, mobile_entity_state}, events::{enemy_despawn_event, EnemyDespawnEvent},
        static_data::{QuestDropDesc, enemy_desc},
        util::OffsetCoordinatesFloat,
    }
};

#[spacetimedb::table(accessor = enemy_despawn_timer, scheduled(enemy_despawn, at = scheduled_at))]
pub struct EnemyDespawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub attacker_entity_id: u64,
    pub entity_id: u64,
}

#[spacetimedb::reducer]
pub fn enemy_spawn_loot(ctx: &ReducerContext, request: EnemySpawnLootRequest) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let enemy_desc = ctx.db.enemy_desc().enemy_type().find(request.enemy_type).unwrap();
    let mut output = Vec::new();
    for stack in &enemy_desc.extracted_item_stacks {
        if let Some(rolled) = stack.roll(ctx, 1) {
            output.push(rolled);
        }
    }
    // Quest drops
    if let Some(mut drops) = QuestDropDesc::roll_enemy(ctx, request.player_entity_id, request.enemy_type) {
        output.append(&mut drops);
    }

    if !output.is_empty() {
        InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            request.player_entity_id,
            &output,
            |x| request.loot_coordinates.distance_to(x),
            true,
            || vec![{ request.loot_coordinates }],
            false,
        )?;

        post_reducer_update_cargo(ctx, request.player_entity_id);
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn enemy_despawn(ctx: &ReducerContext, timer: EnemyDespawnTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    reduce(ctx, timer.entity_id, true);
    Ok(())
}

#[spacetimedb::reducer]
pub fn enemy_despawn_from_mob_monitor(ctx: &ReducerContext, enemy_entity_id: u64) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    reduce(ctx, enemy_entity_id, false);
    Ok(())
}

#[spacetimedb::reducer]
pub fn enemy_despawn_from_mob_monitor_batch(ctx: &ReducerContext, enemy_entity_ids: Vec<u64>) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    for enemy_entity_id in enemy_entity_ids {
        reduce(ctx, enemy_entity_id, false);
    }
    Ok(())
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, emit_despawn_event: bool) {
    // The enemy can be despawned twice if it is killed just as it is unspawned because of the time of day.
    // (one call from the server and one call from the mob monitor)
    // In this case, just despawn it once, with whatever happens first.
    if let Some(enemy) = ctx.db.enemy_state().entity_id().find(entity_id) {
        // Clear all combat sessions involving the dead entity
        ThreatState::clear_all(ctx, entity_id);

        if let Some(mut herd) = ctx.db.herd_state().entity_id().find(enemy.herd_entity_id) {
            herd.current_population -= 1;
            ctx.db.herd_state().entity_id().update(herd);
        }

        ctx.db.action_state().owner_entity_id().delete(entity_id); // this is obsolete, we might want to call a reducer instead
        ctx.db.ability_state().owner_entity_id().delete(entity_id);

        if ContributionState::applies(ctx, entity_id) {
            ContributionState::roll_all(ctx, entity_id);
            ContributionState::clear(ctx, entity_id);
        }

        if let Some(prospecting_lock) = ctx.db.crumb_trail_contribution_lock_state().entity_id().find(enemy.entity_id) {
            if ctx.db.enemy_state().herd_entity_id().filter(enemy.herd_entity_id).count() == 1 {
                // This was the last enemy part of the prospecting herd. Immediately end herd prospecting.
                let crumb_trail = ctx
                    .db
                    .crumb_trail_state()
                    .entity_id()
                    .find(prospecting_lock.crumb_trail_entity_id)
                    .unwrap();
                crumb_trail_clean_up_agent::delete_crumb_trail(ctx, &crumb_trail);
            }
        }

        if emit_despawn_event {
            if let Some(mobile_entity) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
                ctx.db.enemy_despawn_event().insert(EnemyDespawnEvent {
                    entity_id,
                    enemy_type: enemy.enemy_type,
                    despawn_timestamp: unix_ms(ctx.timestamp),
                    location: OffsetCoordinatesFloat {
                        x: mobile_entity.location_x,
                        z: mobile_entity.location_z,
                        dimension: mobile_entity.dimension,
                    },
                    destination: OffsetCoordinatesFloat {
                        x: mobile_entity.destination_x,
                        z: mobile_entity.destination_z,
                        dimension: mobile_entity.dimension,
                    },
                    movement_timestamp: mobile_entity.timestamp,
                });
            }
        }

        delete_entity(ctx, entity_id);
    }
}
