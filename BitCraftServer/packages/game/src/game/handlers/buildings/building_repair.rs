use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::game_util::ToolRequirement;
use crate::{
    game::game_state::{self, game_state_filters},
    messages::{
        action_request::PlayerBuildingRepairRequest,
        components::*,
        events::{building_repair_start_event, BuildingRepairStartEvent},
    },
    unwrap_or_err,
};
use crate::{parameters_desc, tool_type_desc, CharacterStatType, ToolDesc};

pub fn event_delay(ctx: &ReducerContext, player_entity_id: u64) -> Duration {
    let delay = ctx.db.parameters_desc().version().find(0).unwrap().repair_building_duration as f32;
    let building_speed = 1.0 / CharacterStatsState::get_entity_stat(ctx, player_entity_id, CharacterStatType::BuildingSpeed);
    Duration::from_secs_f32(delay * building_speed)
}

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn building_repair_start(ctx: &ReducerContext, request: PlayerBuildingRepairRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = Some(request.building_entity_id);
    let delay = event_delay(ctx, actor_id);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::RepairBuilding,
        target,
        None,
        delay,
        reduce(ctx, actor_id, &request, true),
        request.timestamp,
    );
    if result.is_ok() {
        ctx.db
            .building_repair_start_event()
            .insert(BuildingRepairStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn building_repair(ctx: &ReducerContext, request: PlayerBuildingRepairRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    player_action_helpers::schedule_clear_player_action_on_err(
        actor_id,
        PlayerActionType::RepairBuilding.get_layer(ctx),
        reduce(ctx, actor_id, &request, false),
    )
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, request: &PlayerBuildingRepairRequest, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::RepairBuilding, Some(request.building_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::RepairBuilding, request.timestamp)?;
    }

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building_entity_id = request.building_entity_id;
    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(building_entity_id),
        "Building does not exist"
    );
    if building.distance_to(ctx, &player_coord) > 2 {
        return Err("Too far".into());
    }

    let building_max_health = HealthState::max_building_health(ctx, building_entity_id);
    let building_health = unwrap_or_err!(
        ctx.db.health_state().entity_id().find(building_entity_id),
        "Unable to querry building health"
    )
    .health;

    if building_health == building_max_health {
        return Err("Building is already at full health".into());
    }

    if !dry_run {
        let tool_requirement = ToolRequirement {
            tool_type: ctx.db.tool_type_desc().skill_id().find(0).unwrap().id,
            level: 0,
            power: 0,
        };
        let tool = ToolDesc::get_required_tool(ctx, actor_id, &tool_requirement)?;

        // From depleting claim shield: repair building only
        if !HealthState::add_building_health(ctx, building_entity_id, tool.power as f32) {
            return Err("Unable to repair building".into());
        }

        // building health is still set to the previous value
        if building_health + tool.power as f32 >= building_max_health {
            PlayerActionState::success(
                ctx,
                actor_id,
                PlayerActionType::None,
                PlayerActionType::RepairBuilding.get_layer(ctx),
                0,
                None,
                None,
                request.timestamp,
            );
        }
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
