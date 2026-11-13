use std::{collections::HashMap, time::Duration};

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents, enemy_state, health_state,
    messages::{
        authentication::ServerIdentity,
        components::{ActiveBuffState, ThreatState},
        static_data::*,
    },
    unwrap_or_return,
};

#[spacetimedb::table(accessor = enemy_regen_loop_timer, scheduled(enemy_regen_agent_loop, at = scheduled_at))]
pub struct EnemyRegenLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().enemy_regen_tick_millis as u64;
    let mut count = 0;
    for mut timer in ctx.db.enemy_regen_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(tick_length).into();
        ctx.db.enemy_regen_loop_timer().scheduled_id().update(timer);
        log::info!("enemy regen agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one EnemyRegenLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = ctx.db.parameters_desc().version().find(0).unwrap().enemy_regen_tick_millis as u64;
    ctx.db
        .enemy_regen_loop_timer()
        .try_insert(EnemyRegenLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn enemy_regen_agent_loop(ctx: &ReducerContext, _timer: EnemyRegenLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to enemy_regen agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let params = unwrap_or_return!(ctx.db.parameters_desc().version().find(0), "Failed to get ParametersDesc");
    let min_seconds_to_passive_regen_health = params.min_seconds_to_passive_regen_health as u64;

    regen_enemies(ctx, min_seconds_to_passive_regen_health);
}

fn regen_enemies(ctx: &ReducerContext, min_seconds_to_passive_regen_health: u64) {
    let enemy_descs: HashMap<EnemyType, EnemyDesc> = ctx.db.enemy_desc().iter().map(|e| (EnemyType::to_enum(e.enemy_type), e)).collect();

    let mut regen_buffs = Vec::new();
    let mut regen_buffs_values = Vec::new();

    // Collect all buff ids and regen values of any buff with a "ActiveHealthRegenRate" stat update
    for (buff_id, buff_value) in ctx.db.buff_desc().iter().map(|buff| {
        (
            buff.id,
            buff.stats
                .iter()
                .filter_map(|s| {
                    if s.id == CharacterStatType::ActiveHealthRegenRate {
                        Some(s.value)
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or(0.0),
        )
    }) {
        if buff_value != 0.0 {
            regen_buffs.push(buff_id);
            regen_buffs_values.push(buff_value);
        }
    }

    for enemy in ctx.db.enemy_state().iter() {
        regen_health(
            ctx,
            min_seconds_to_passive_regen_health,
            enemy.entity_id,
            &enemy_descs[&enemy.enemy_type()],
            &regen_buffs,
            &regen_buffs_values,
        );
    }
}

fn regen_health(
    ctx: &ReducerContext,
    min_seconds_to_passive_regen_health: u64,
    enemy_entity_id: u64,
    enemy_desc: &EnemyDesc,
    regen_buffs: &[i32],
    regen_buffs_values: &[f32],
) {
    let mut health_state = unwrap_or_return!(
        ctx.db.health_state().entity_id().find(enemy_entity_id),
        "Failed to get HealthState for enemy with id {}",
        enemy_entity_id
    );
    let previous_health = health_state.health;

    let active_health_regen = ActiveBuffState::get_enemy_health_regen(ctx, enemy_entity_id, regen_buffs, regen_buffs_values);
    if active_health_regen != 0.0 {
        health_state.add_health_delta_clamped(active_health_regen, 0f32, enemy_desc.max_health as f32, ctx.timestamp);
    }

    if active_health_regen >= 0.0 && // Had to add this line because of a bug in spacetimedb (elapsed() of 0ns panics)
        !health_state.is_incapacitated_self()
        && ctx.timestamp.duration_since(health_state.last_health_decrease_timestamp).unwrap().as_secs() >= min_seconds_to_passive_regen_health
        && !ThreatState::in_combat(ctx, health_state.entity_id)
    {
        health_state.add_health_delta_clamped(enemy_desc.health_regen_quantity, 0f32, enemy_desc.max_health as f32, ctx.timestamp);
    }

    if health_state.health != previous_health {
        ctx.db.health_state().entity_id().update(health_state);
    }
}
