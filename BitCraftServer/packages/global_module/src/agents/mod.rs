use crate::game::handlers::authentication::has_role;
use crate::messages::authentication::Role;
use crate::messages::generic::config;
use crate::parameters_desc;
use spacetimedb::{log, ReducerContext};

pub mod chat_cleanup_agent;
pub mod empire_decay_agent;
pub mod empire_siege_agent;

pub fn should_run(ctx: &ReducerContext) -> bool {
    if let Some(config) = ctx.db.config().version().find(0) {
        return config.agents_enabled;
    }

    false
}

pub fn init(ctx: &ReducerContext) {
    if ctx.db.parameters_desc().version().find(0).is_some() {
        empire_decay_agent::init(ctx);
        empire_siege_agent::init(ctx);
        chat_cleanup_agent::init(ctx);

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
    empire_decay_agent::update_timer(ctx);
    empire_siege_agent::update_timer(ctx);
    chat_cleanup_agent::update_timer(ctx);
    Ok(())
}
