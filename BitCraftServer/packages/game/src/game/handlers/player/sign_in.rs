use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        entities::buff,
        game_state::{self, game_state_filters},
        handlers::{
            authentication::has_role, player_vault::deployable_hide::hide_deployable_timer, queue::end_grace_period::end_grace_period_timer,
        },
    },
    messages::{
        action_request::PlayerSignInRequest,
        authentication::Role,
        components::*,
        generic::{region_control_info, RegionControlInfo, RegionSignInParameters},
    },
    parameters_desc, unwrap_or_err,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn sign_in(ctx: &ReducerContext, _request: PlayerSignInRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, false)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let now_ms = ctx.timestamp;

    log::info!("[{:?}] Signin {}, {:?}", now_ms, actor_id, &ctx.sender.to_hex());

    let region_sign_in_parameters = unwrap_or_err!(RegionSignInParameters::get(ctx), "Failed to get RegionSignInParameters");
    if region_sign_in_parameters.is_signing_in_blocked && !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err(format!("Server is unavailable at this time, please try again later."));
    }

    let region_id = region_sign_in_parameters.region_id;
    let region_control = ctx
        .db
        .region_control_info()
        .region_id()
        .find(region_id)
        .unwrap_or(RegionControlInfo {
            region_id,
            initialized: false,
            allow_players: false,
            allow_player_spawns: false,
        });
    if !region_control.initialized {
        //Prevent anyone from logging in if there's no world
        return Err("This region does not currently accept sign ins".into());
    }

    if ctx.db.signed_in_player_state().entity_id().find(actor_id).is_some() {
        // Reconnect of an already signed-in player; the session continues.
        return Ok(());
    }

    let user = unwrap_or_err!(ctx.db.user_state().identity().find(&ctx.sender), "No user found");
    if !user.can_sign_in {
        return Err(format!("You must join the queue first."));
    }

    let mut player = unwrap_or_err!(ctx.db.player_state().entity_id().find(actor_id), "Invalid player id");

    let mobile_entity_state = ctx.db.mobile_entity_state().entity_id().find(actor_id).unwrap();
    if ctx
        .db
        .terrain_chunk_state()
        .chunk_index()
        .find(mobile_entity_state.chunk_index)
        .is_none()
    {
        let _ = game_state_filters::teleport_to(ctx, actor_id, player.teleport_location.location.into(), true, 0.0);
    }

    if let Some(network) = ctx
        .db
        .dimension_description_state()
        .dimension_id()
        .find(mobile_entity_state.dimension)
    {
        InteriorPlayerCountState::inc(ctx, network.dimension_network_entity_id);
    }

    let moderations = ctx.db.user_moderation_state().target_identity().filter(&ctx.sender);

    for moderation in moderations {
        if moderation.user_moderation_policy == UserModerationPolicy::PermanentBlockLogin {
            return Err(format!("Your account is blocked from logging in."));
        }

        if moderation.user_moderation_policy == UserModerationPolicy::TemporaryBlockLogin {
            if moderation.expiration_time > ctx.timestamp {
                return Err(format!("Your account is blocked from logging in."));
            }
        }
    }

    //Cancel all scheduled grace periods
    ctx.db.end_grace_period_timer().identity().delete(ctx.sender);

    player.signed_in = true;
    player.session_start_timestamp = game_state::unix(ctx.timestamp);
    player.sign_in_timestamp = game_state::unix(ctx.timestamp);

    TravelerTaskCreditState::refresh_player_credits(ctx, actor_id);

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    inventory.unlock_all_pockets();
    ctx.db.inventory_state().entity_id().update(inventory);

    // restart timestamps
    let active_buff_state = unwrap_or_err!(
        ctx.db.active_buff_state().entity_id().find(&actor_id),
        "Player has no active buff state."
    );

    let mut active_buff_state = active_buff_state.clone();

    // restart buff timestamp
    active_buff_state.restart_all_buffs(ctx);

    // Refresh Innerlight buff with a short login immunity
    let innerlight_buff_duration = ctx.db.parameters_desc().version().find(&0).unwrap().sign_in_aggro_immunity;
    active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);

    PlayerState::collect_stats_with_uncommited_buffs(ctx, &active_buff_state);

    // Update the last action timestamp on login so the auto-logout agent doesn't kick the player out immediately
    let mut player_action_state = PlayerActionState::get_state(ctx, &actor_id, &PlayerActionLayer::Base).unwrap();
    player_action_state.start_time = game_state::unix_ms(ctx.timestamp);
    ctx.db.player_action_state().auto_id().update(player_action_state);

    ctx.db.active_buff_state().entity_id().update(active_buff_state);

    ctx.db.player_state().entity_id().update(player);

    PlayerState::collect_stats(ctx, actor_id);

    AlertState::on_sign_in(ctx, actor_id);

    ctx.db
        .signed_in_player_state()
        .try_insert(SignedInPlayerState { entity_id: actor_id })?;

    let mut mobile = ctx.db.mobile_entity_state().entity_id().find(&actor_id).unwrap();
    mobile.destination_x = mobile.location_x;
    mobile.destination_z = mobile.location_z;
    mobile.is_walking = false;
    buff::deactivate_sprint(ctx, mobile.entity_id);
    mobile.timestamp = game_state::unix_ms(ctx.timestamp);
    ctx.db.mobile_entity_state().entity_id().update(mobile);

    // Claim pending collectibles
    if let Some(unclaimed_collectibles) = ctx.db.unclaimed_collectibles_state().identity().find(&user.identity) {
        let mut vault = ctx.db.vault_state().entity_id().find(&actor_id).unwrap();
        for collectible_id in unclaimed_collectibles.collectibles {
            let _ = vault.add_collectible(ctx, collectible_id, false);
        }
        ctx.db.vault_state().entity_id().update(vault);
        ctx.db.unclaimed_collectibles_state().identity().delete(&user.identity);
    }

    // Display hidden deployables
    let deployable_ids: Vec<_> = ctx
        .db
        .deployable_state_v2()
        .owner_id()
        .filter(actor_id)
        .map(|deployable| (deployable.entity_id, deployable.hidden))
        .collect();
    for (entity_id, hidden) in deployable_ids {
        //delete timer that sets deployable hidden
        ctx.db.hide_deployable_timer().entity_id().delete(&entity_id);

        //set hidden deployable visible
        if hidden {
            let mut deployable = ctx.db.deployable_state_v2().entity_id().find(&entity_id).unwrap();
            deployable.hidden = false;
            ctx.db.deployable_state_v2().entity_id().update(deployable);
        }
    }

    log::info!("Signin complete");

    Ok(())
}
