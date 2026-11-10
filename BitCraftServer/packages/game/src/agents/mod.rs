use crate::game::handlers::authentication::has_role;
use crate::messages::authentication::Role;
use crate::messages::generic::config;
use crate::parameters_desc;
use spacetimedb::{log, ReducerContext};

mod auto_logout_agent;
pub mod building_decay_agent;
pub mod chat_cleanup_agent;
pub mod crumb_trail_clean_up_agent;
pub mod day_night_agent;
pub mod duel_agent;
pub mod enemy_regen_agent;
pub mod environment_debuff_agent;
pub mod growth_agent;
pub mod npc_ai_agent;
pub mod player_housing_income_agent;
pub mod player_regen_agent;
pub mod player_session_log_cleanup_agent;
pub mod region_population_agent;
pub mod rent_collector_agent;
pub mod resources_regen;
pub mod starving_agent;
pub mod storage_log_cleanup_agent;
pub mod teleportation_energy_regen_agent;
pub mod trade_sessions_agent;
pub mod traveler_task_agent;

pub fn should_run(ctx: &ReducerContext) -> bool {
    if let Some(config) = ctx.db.config().version().find(&0) {
        return config.agents_enabled;
    }

    return false;
}

pub fn init(ctx: &ReducerContext) {
    if ctx.db.parameters_desc().version().find(&0).is_some() {
        enemy_regen_agent::init(ctx);
        player_regen_agent::init(ctx);
        teleportation_energy_regen_agent::init(ctx);
        starving_agent::init(ctx);
        environment_debuff_agent::init(ctx);
        resources_regen::init(ctx);
        building_decay_agent::init(ctx);
        npc_ai_agent::init(ctx);
        auto_logout_agent::init(ctx);
        duel_agent::init(ctx);
        trade_sessions_agent::init(ctx);
        traveler_task_agent::init(ctx);
        growth_agent::init(ctx);
        chat_cleanup_agent::init(ctx);
        rent_collector_agent::init(ctx);
        player_housing_income_agent::init(ctx);
        region_population_agent::init(ctx);
        storage_log_cleanup_agent::init(ctx);
        crumb_trail_clean_up_agent::init(ctx);
        player_session_log_cleanup_agent::init(ctx);

        log::info!("Initialized agents with parameter values.");
    } else {
        log::error!("Could not load parameter values to initialize agents.");
    }
}

#[spacetimedb::reducer]
pub fn update_scheduled_timers_from_static_data(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }
    log::info!("Updating scheduled timers with values from static data");
    building_decay_agent::update_timer(ctx);
    enemy_regen_agent::update_timer(ctx);
    environment_debuff_agent::update_timer(ctx);
    growth_agent::update_timer(ctx);
    player_regen_agent::update_timer(ctx);
    resources_regen::update_timer(ctx);
    starving_agent::update_timer(ctx);
    storage_log_cleanup_agent::update_timer(ctx);
    player_session_log_cleanup_agent::update_timer(ctx);
    Ok(())
}
