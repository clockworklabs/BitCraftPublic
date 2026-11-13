use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::messages::events::*;
use crate::{
    game::{
        game_state,
        reducer_helpers::{deployable_helpers, player_action_helpers},
        terrain_chunk::TerrainChunkCache,
    },
    messages::{action_request::DeployableDeployRequest, components::*, static_data::*},
    unwrap_or_err,
};

use super::collectible_activate;

pub fn event_delay(ctx: &ReducerContext, actor_id: u64, request: &DeployableDeployRequest) -> Duration {
    if let Some(vault) = ctx.db.vault_state().entity_id().find(actor_id) {
        let collectibles = &vault.collectibles;
        let collectible_id = collectibles.get(request.vault_index as usize).unwrap().id;
        if let Some(collectible_desc) = ctx.db.collectible_desc().id().find(collectible_id) {
            if collectible_desc.collectible_type == CollectibleType::Deployable {
                let deployable_description = ctx.db.deployable_desc().deploy_from_collectible_id().find(collectible_id).unwrap();
                return Duration::from_secs_f32(deployable_description.deploy_time);
            }
        }
    }
    Duration::ZERO
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_deploy_start(ctx: &ReducerContext, request: DeployableDeployRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let delay = event_delay(ctx, actor_id, &request);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::DeployDeployable,
        None,
        None,
        delay,
        reduce(ctx, actor_id, &request, false, true),
        game_state::unix_ms(ctx.timestamp),
    );
    if result.is_ok() {
        ctx.db
            .deployable_deploy_start_event()
            .insert(DeployableDeployStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_deploy(ctx: &ReducerContext, request: DeployableDeployRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::DeployDeployable.get_layer(ctx),
        reduce(ctx, actor_id, &request, true, false),
    )
}

fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    request: &DeployableDeployRequest,
    validate_action: bool,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if !dry_run && validate_action {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::DeployDeployable, None)?;
    }

    if ctx.db.mounting_state().entity_id().find(actor_id).is_some() {
        return Err("Can't place a deployable while in a deployable".into());
    }

    collectible_activate::reduce(ctx, actor_id, request.vault_index, true, dry_run)?;

    let mut terrain_cache = TerrainChunkCache::empty();

    let vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(actor_id), "Vault not initialized");
    let collectibles = vault.collectibles;
    let collectible_id = collectibles.get(request.vault_index as usize).unwrap().id;

    deployable_helpers::deploy_deployable(
        ctx,
        &mut terrain_cache,
        actor_id,
        collectible_id,
        request.direction,
        request.coord.into(),
        dry_run,
    )
}
