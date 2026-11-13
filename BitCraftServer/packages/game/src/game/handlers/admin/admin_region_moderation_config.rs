use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        moderation_config::{region_moderation_config_state, RegionModerationConfigState},
    },
};

#[spacetimedb::reducer]
pub fn admin_update_region_moderation_config(
    ctx: &ReducerContext,
    max_messages_per_time_period: u32,
    rate_limit_window_sec: i32,
    new_account_min_playtime_sec: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    // Upsert pattern: delete existing, insert new
    ctx.db.region_moderation_config_state().id().delete(0);
    ctx.db.region_moderation_config_state().insert(RegionModerationConfigState {
        id: 0,
        max_messages_per_time_period,
        rate_limit_window_sec,
        new_account_min_playtime_sec,
    });

    log::info!(
        "[Admin] Updated region moderation config: max_msgs={}, window={}s, min_playtime={}s",
        max_messages_per_time_period,
        rate_limit_window_sec,
        new_account_min_playtime_sec,
    );
    Ok(())
}
