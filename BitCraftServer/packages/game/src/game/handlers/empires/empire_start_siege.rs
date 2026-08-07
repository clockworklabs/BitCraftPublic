use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::ReducerContext;

use crate::{
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::{deployable_helpers::deploy_standalone_deployable, player_action_helpers},
        terrain_chunk::TerrainChunkCache,
    },
    inter_module::*,
    messages::{
        components::*, empire_shared::*, game_util::ItemStack, inter_module::*, static_data::*, util::OffsetCoordinatesSmallMessage,
    },
    unwrap_or_err, unwrap_or_return,
};

use super::empires_shared::validate_siege_distance;

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_siege_depleted_watchtower(ctx: &ReducerContext, request: EmpireStartSiegeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;
    let player_location = game_state_filters::coordinates_any(ctx, actor_id);
    let watchtower_location = game_state_filters::coordinates(ctx, request.building_entity_id);
    if player_location.distance_to(watchtower_location) > 2 {
        return Err("Too far".into());
    }

    let defending_node = unwrap_or_err!(
        ctx.db.empire_node_state().entity_id().find(&request.building_entity_id),
        "This building cannot be sieged"
    );
    if defending_node.energy > 0 {
        return Err("This watchtower still has supplies".into());
    }

    let (supplies, supply_cargo_id) = EmpireNodeSiegeState::consume_player_cargo(ctx, request.building_entity_id, actor_id)?;
    send_message(ctx, actor_id, request.building_entity_id, 0, supplies, supply_cargo_id, true);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_deploy_siege_engine_start(ctx: &ReducerContext, request: EmpireStartSiegeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let deployable_id = 1000; // Siege Engine
    let deployable_description = ctx.db.deployable_desc().id().find(&deployable_id).unwrap();
    let delay = Duration::from_secs_f32(deployable_description.deploy_time);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::DeployDeployable,
        Some(deployable_id as u64),
        None,
        delay,
        empire_deploy_siege_engine_reduce(ctx, actor_id, request.coord, request.direction, request.building_entity_id, true),
        game_state::unix_ms(ctx.timestamp),
    )
}

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_deploy_siege_engine(ctx: &ReducerContext, request: EmpireStartSiegeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::DeployDeployable.get_layer(ctx),
        empire_deploy_siege_engine_reduce(ctx, actor_id, request.coord, request.direction, request.building_entity_id, false),
    )
}

fn empire_deploy_siege_engine_reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    coord: OffsetCoordinatesSmallMessage,
    direction: i32,
    building_entity_id: u64,
    dry_run: bool,
) -> Result<(), String> {
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::DeployDeployable, Some(1000))?;
    }

    validate_siege_distance(
        ctx,
        building_entity_id,
        coord.into(),
        "You need to deploy this siege engine",
        0,
    )?;

    if EmpireNodeSiegeState::has_active_siege(ctx, building_entity_id) {
        return Err("There is already a siege in progress".into());
    }
    EmpireNodeSiegeState::validate_action(ctx, actor_id, building_entity_id)?;
    EmpireNodeSiegeState::validate_building(ctx, building_entity_id)?;

    let mut terrain_cache = TerrainChunkCache::empty();
    let deployable_id = 1000; // Siege Engine

    let deployable_entity_id = game_state::create_entity(ctx);

    deploy_standalone_deployable(
        ctx,
        &mut terrain_cache,
        actor_id,
        deployable_entity_id,
        deployable_id,
        direction,
        coord.into(),
        dry_run,
    )?;

    if !dry_run {
        let (supplies, supply_cargo_id) = EmpireNodeSiegeState::consume_player_cargo(ctx, building_entity_id, actor_id)?;
        send_message(
            ctx,
            actor_id,
            building_entity_id,
            deployable_entity_id,
            supplies,
            supply_cargo_id,
            false,
        );
    }

    Ok(())
}

pub fn send_message(
    ctx: &ReducerContext,
    player_entity_id: u64,
    building_entity_id: u64,
    deployable_entity_id: u64,
    supplies: i32,
    supply_cargo_id: i32,
    is_depleted_watchtower: bool,
) {
    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireStartSiege(EmpireStartSiegeMsg {
            building_coord: game_state_filters::coordinates(ctx, building_entity_id).into(),
            player_entity_id,
            building_entity_id,
            deployable_entity_id,
            supplies,
            supply_cargo_id,
            is_depleted_watchtower,
        }),
        InterModuleDestination::Global,
    );
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireStartSiegeMsg, error: Option<String>) {
    if error.is_some() {
        //Add supply cargo if remote call fails
        let mut player_inventory = unwrap_or_return!(
            InventoryState::get_player_inventory(ctx, request.player_entity_id),
            "Player has no inventory"
        );
        let supplies = vec![ItemStack::single_cargo(request.supply_cargo_id)];
        player_inventory.add_multiple_with_overflow(ctx, &supplies);
        ctx.db.inventory_state().entity_id().update(player_inventory);

        if request.deployable_entity_id > 0 {
            ctx.db.mobile_entity_state().entity_id().delete(&request.deployable_entity_id);
            ctx.db.deployable_state_v2().entity_id().delete(&request.deployable_entity_id);
        }

        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
