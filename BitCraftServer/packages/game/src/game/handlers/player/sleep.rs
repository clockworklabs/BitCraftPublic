use bitcraft_macro::feature_gate;
use std::time::Duration;

use entities::resource_clump::SmallHexTile;
use spacetimedb::{ReducerContext, Table};

use crate::{
    building_desc,
    game::{
        entities,
        game_state::{self, game_state_filters},
    },
    messages::{action_request::PlayerSleepRequest, components::*, events::*, static_data::BuildingCategory},
    unwrap_or_err,
};

pub fn event_delay(_actor_id: u64, _request: &PlayerSleepRequest) -> Duration {
    Duration::from_secs_f32(1.0)// wait 1 second before start gaining health/stamina - roughly the animation time
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn sleep(ctx: &ReducerContext, request: PlayerSleepRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let result = reduce(ctx, actor_id);
    if result.is_ok() {
        ctx.db.sleep_event().insert(SleepEvent { actor_id, request });
    }
    result
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    can_sleep(ctx, game_state_filters::coordinates_any(ctx, actor_id))?;

    PlayerActionState::success(
        ctx,
        actor_id,
        PlayerActionType::Sleep,
        PlayerActionType::Sleep.get_layer(ctx),
        0,
        None,
        None,
        game_state::unix_ms(ctx.timestamp),
    );

    Ok(())
}

pub fn can_sleep(ctx: &ReducerContext, coordinates: SmallHexTile) -> Result<(), String> {
    let building = unwrap_or_err!(
        game_state_filters::building_at_coordinates(ctx, &coordinates),
        "No building at this location"
    );

    let building_description = unwrap_or_err!(
        ctx.db.building_desc().id().find(building.building_description_id),
        "Building doesn't exist in the static data"
    );

    if !building_description.has_category(ctx, BuildingCategory::Bed) {
        return Err("You cannot sleep in there".into());
    }
    Ok(())
}
