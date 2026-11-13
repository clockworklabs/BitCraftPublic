use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::game::reducer_helpers::health_helpers::update_health_and_check_death;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::ThreatState;
use crate::{
    agents,
    messages::{authentication::ServerIdentity, components::CharacterStatsState, static_data::*},
    unwrap_or_continue, unwrap_or_return, SatiationState,
};
use crate::{character_stats_state, health_state, signed_in_player_state, stamina_state};

#[spacetimedb::table(accessor = player_regen_loop_timer, scheduled(player_regen_agent_loop, at = scheduled_at))]
pub struct PlayerRegenLoopTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

pub fn update_timer(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    let mut count = 0;
    for mut timer in ctx.db.player_regen_loop_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_millis(params.player_regen_tick_millis as u64).into();
        ctx.db.player_regen_loop_timer().scheduled_id().update(timer);
        log::info!("player regen agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one PlayerRegenLoopTimer running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    ctx.db
        .player_regen_loop_timer()
        .try_insert(PlayerRegenLoopTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(params.player_regen_tick_millis as u64).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn player_regen_agent_loop(ctx: &ReducerContext, _timer: PlayerRegenLoopTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to player_regen agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let params = ctx.db.parameters_desc().version().find(0).unwrap();

    let min_seconds_to_passive_regen_health = params.min_seconds_to_passive_regen_health as u64;
    let min_seconds_to_passive_regen_stamina = params.min_seconds_to_passive_regen_stamina as u64;

    regen_players(ctx, min_seconds_to_passive_regen_health, min_seconds_to_passive_regen_stamina);
}

fn regen_players(ctx: &ReducerContext, min_seconds_to_passive_regen_health: u64, min_seconds_to_passive_regen_stamina: u64) {
    for signed_in_player_state in ctx.db.signed_in_player_state().iter() {
        let character_stats_state = unwrap_or_continue!(
            ctx.db.character_stats_state().entity_id().find(signed_in_player_state.entity_id),
            "Failed to get CharacterStatsState for player with id {}",
            signed_in_player_state.entity_id
        );

        regen_health(ctx, min_seconds_to_passive_regen_health, &character_stats_state);
        regen_stamina(ctx, min_seconds_to_passive_regen_stamina, &character_stats_state);
        regen_satiation(ctx, &character_stats_state);
    }
}

fn regen_health(ctx: &ReducerContext, min_seconds_to_passive_regen_health: u64, character_stats_state: &CharacterStatsState) {
    let mut health_state = unwrap_or_return!(
        ctx.db.health_state().entity_id().find(character_stats_state.entity_id),
        "Failed to get HealthState for player with id {}",
        character_stats_state.entity_id
    );

    if health_state.is_incapacitated_self() {
        return;
    }

    let mut health_regen = character_stats_state.get(CharacterStatType::ActiveHealthRegenRate);

    if ctx
        .timestamp
        .duration_since(health_state.last_health_decrease_timestamp)
        .unwrap_or_default()
        .as_secs()
        >= min_seconds_to_passive_regen_health
        && !ThreatState::in_combat(ctx, health_state.entity_id)
    {
        health_regen += character_stats_state.get(CharacterStatType::PassiveHealthRegenRate);
    }

    if health_state.add_health_delta_clamped(
        health_regen,
        0f32,
        character_stats_state.get(CharacterStatType::MaxHealth),
        ctx.timestamp,
    ) {
        let mut terrain_cache = TerrainChunkCache::empty();
        update_health_and_check_death(ctx, &mut terrain_cache, health_state, character_stats_state.entity_id, None);
    }
}

fn regen_stamina(ctx: &ReducerContext, min_seconds_to_passive_regen_stamina: u64, character_stats_state: &CharacterStatsState) {
    let mut stamina_state = unwrap_or_return!(
        ctx.db.stamina_state().entity_id().find(character_stats_state.entity_id),
        "Failed to get StaminaState for player with id {}",
        character_stats_state.entity_id
    );

    let mut stamina_regen = character_stats_state.get(CharacterStatType::ActiveStaminaRegenRate);

    if ctx
        .timestamp
        .duration_since(stamina_state.last_stamina_decrease_timestamp)
        .unwrap_or_default()
        .as_secs()
        >= min_seconds_to_passive_regen_stamina
    {
        stamina_regen += character_stats_state.get(CharacterStatType::PassiveStaminaRegenRate);
    }

    if stamina_state.add_stamina_delta_clamped(
        stamina_regen,
        0f32,
        character_stats_state.get(CharacterStatType::MaxStamina),
        ctx.timestamp,
    ) {
        ctx.db.stamina_state().entity_id().update(stamina_state);
    }
}

fn regen_satiation(ctx: &ReducerContext, character_stats_state: &CharacterStatsState) {
    SatiationState::add_player_satiation(
        ctx,
        character_stats_state.entity_id,
        character_stats_state.get(CharacterStatType::SatiationRegenRate),
    );
}
