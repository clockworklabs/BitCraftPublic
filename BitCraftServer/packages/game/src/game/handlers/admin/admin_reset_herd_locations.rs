use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        handlers::{authentication::has_role, server::enemy_despawn::enemy_despawn_from_mob_monitor},
        location_cache::location_cache,
        terrain_chunk::TerrainChunkCache,
    },
    messages::{
        authentication::Role,
        components::{attached_herds_state, enemy_state, herd_state},
        static_data::enemy_ai_params_desc,
    },
};

#[spacetimedb::reducer]
pub fn admin_reset_herd_locations(ctx: &ReducerContext, enemy_ai_params_desc_id: i32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // delete all enemies of this type
    delete_herds(ctx, enemy_ai_params_desc_id);

    // remove deleted herds from buildings
    update_attached_herds(ctx);

    // delete enemies whose herds have been deleted
    delete_unaffiliated_enemies(ctx);

    // spawn new herds for all missing enemy types
    spawn_herds(ctx);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_reset_all_herd_locations(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // delete all herds of all enemy types
    for entry in ctx.db.enemy_ai_params_desc().iter() {
        delete_herds(ctx, entry.id);
    }

    // remove deleted herds from buildings
    update_attached_herds(ctx);

    // delete enemies whose herds have been deleted
    delete_unaffiliated_enemies(ctx);

    // spawn new herds for all missing enemy types
    spawn_herds(ctx);

    Ok(())
}

fn delete_herds(ctx: &ReducerContext, enemy_ai_params_desc_id: i32) {
    ctx.db.herd_state().enemy_ai_params_desc_id().delete(enemy_ai_params_desc_id);
}

fn update_attached_herds(ctx: &ReducerContext) {
    for mut attached_herd in ctx.db.attached_herds_state().iter() {
        let mut updated = false;
        for i in (0..attached_herd.herds_entity_ids.len()).rev() {
            if ctx.db.herd_state().entity_id().find(attached_herd.herds_entity_ids[i]).is_none() {
                updated = true;
                attached_herd.herds_entity_ids.remove(i);
            }
        }
        if updated {
            if attached_herd.herds_entity_ids.is_empty() {
                ctx.db.attached_herds_state().entity_id().delete(attached_herd.entity_id);
            } else {
                ctx.db.attached_herds_state().entity_id().update(attached_herd);
            }
        }
    }
}

fn delete_unaffiliated_enemies(ctx: &ReducerContext) {
    for enemy in ctx.db.enemy_state().iter() {
        if ctx.db.herd_state().entity_id().find(enemy.herd_entity_id).is_none() {
            let _ = enemy_despawn_from_mob_monitor(ctx, enemy.entity_id);
        }
    }
}

fn spawn_herds(ctx: &ReducerContext) {
    let mut terrain_cache = TerrainChunkCache::fetch(ctx);

    let mut location_cache = ctx.db.location_cache().version().find(0).unwrap();
    location_cache.build_enemy_spawn_locations(ctx, &mut terrain_cache);
    ctx.db.location_cache().version().update(location_cache);
}
