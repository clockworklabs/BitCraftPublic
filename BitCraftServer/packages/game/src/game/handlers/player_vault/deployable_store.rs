use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::ReducerContext;

use crate::{
    deployable_desc,
    game::{
        game_state::{self},
        reducer_helpers::{deployable_helpers::store_deployable, player_action_helpers},
    },
    inter_module::send_inter_module_message,
    messages::{action_request::DeployableStoreRequest, components::*},
    unwrap_or_err,
};

pub fn event_delay(ctx: &ReducerContext, request: &DeployableStoreRequest) -> Duration {
    if let Some(deployable) = ctx
        .db
        .deployable_collectible_state()
        .deployable_entity_id()
        .find(&request.deployable_entity_id)
    {
        let deployable_description = ctx.db.deployable_desc().id().find(&deployable.deployable_desc_id).unwrap();
        let duration = match request.remotely {
            true => 30f32,
            false => deployable_description.deploy_time,
        };

        return Duration::from_secs_f32(duration);
    }
    Duration::ZERO
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_store_start(ctx: &ReducerContext, request: DeployableStoreRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let delay = event_delay(ctx, &request);

    let target = Some(request.deployable_entity_id);
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::StoreDeployable,
        target,
        None,
        delay,
        reduce(ctx, actor_id, &request, true),
        game_state::unix_ms(ctx.timestamp),
    )
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_store(ctx: &ReducerContext, request: DeployableStoreRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::StoreDeployable.get_layer(ctx),
        reduce(ctx, actor_id, &request, false),
    )
}

fn reduce(ctx: &ReducerContext, actor_id: u64, request: &DeployableStoreRequest, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::StoreDeployable, Some(request.deployable_entity_id))?;
    }

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if request.remotely {
        return reduce_recover(ctx, actor_id, request, dry_run);
    }

    let deployable_state = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(&request.deployable_entity_id),
        "Deployable does not exist."
    );

    // We should never attempt storing already stored deployables
    let _deployable_coordinates = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(request.deployable_entity_id),
        "Deployable is already stored"
    )
    .coordinates();

    // make sure the actor is the deployable owner
    if deployable_state.owner_id != actor_id {
        return Err("You are not the owner of this deployable.".into());
    }

    // make sure the actor is close to the deployable
    let player_coordinates =
        unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Unknown Player Location").coordinates();
    let deployable_coordinates = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(&request.deployable_entity_id),
        "Unknown Deployable Location"
    )
    .coordinates();

    if player_coordinates.distance_to(deployable_coordinates) > 2 {
        return Err("Too far away".into());
    }

    return store_deployable(ctx, actor_id, request.deployable_entity_id, dry_run);
}

fn reduce_recover(ctx: &ReducerContext, actor_id: u64, request: &DeployableStoreRequest, dry_run: bool) -> Result<(), String> {
    if let Some(deployable_state) = ctx.db.deployable_state_v2().entity_id().find(&request.deployable_entity_id) {
        // Deployable is on this region
        // make sure the actor is the deployable owner
        if deployable_state.owner_id != actor_id {
            return Err("You are not the owner of this deployable.".into());
        }

        return store_deployable(ctx, actor_id, request.deployable_entity_id, dry_run);
    } else if !dry_run {
        //Deployable is on another region
        let collectible = unwrap_or_err!(
            ctx.db
                .deployable_collectible_state()
                .deployable_entity_id()
                .find(request.deployable_entity_id),
            "Deployable doesn't exist"
        );

        if collectible.owner_entity_id != actor_id {
            return Err("You are not the owner of this deployable.".into());
        }

        //We don't know which region deployable is on, so we just blast messages to all regions and see if one of them succeeds
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::RecoverDeployable(crate::messages::inter_module::RecoverDeployableMsg {
                player_entity_id: actor_id,
                deployable_entity_id: request.deployable_entity_id,
                deployable_desc_id: collectible.deployable_desc_id,
            }),
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
        return Ok(());
    }
    Ok(())
}
