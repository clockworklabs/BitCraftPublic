use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        game_state::{self},
        reducer_helpers::timer_helpers::now_plus_secs,
    },
    messages::{
        action_request::PlayerHousingEvictPlayerRequest, authentication::ServerIdentity, components::*, static_data::parameters_desc,
    },
    params, unwrap_or_err,
};

#[spacetimedb::table(accessor = player_housing_evict_player_timer, public, scheduled(player_housing_evict_player_complete, at = scheduled_at), index(accessor = player_entity_id, btree(columns = [player_entity_id])))]
pub struct PlayerHousingEvictPlayerTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub building_entity_id: u64,
    pub player_entity_id: u64,
}

impl PlayerHousingEvictPlayerTimer {
    pub fn schedule(ctx: &ReducerContext, building_entity_id: u64, player_entity_id: u64) {
        let eviction_timeout_secs = 60 * params!(ctx).player_housing_eviction_time_minutes as u64;

        ctx.db.player_housing_evict_player_timer().insert(PlayerHousingEvictPlayerTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(eviction_timeout_secs, ctx.timestamp), // not repeating
            building_entity_id,
            player_entity_id,
        });
    }
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn player_housing_evict_player_complete(ctx: &ReducerContext, timer: PlayerHousingEvictPlayerTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to player_housing_evict_player_complete");
        return;
    }

    if let Some(mut player_housing) = ctx.db.player_housing_state().entity_id().find(timer.player_entity_id) {
        if player_housing.entrance_building_entity_id == timer.building_entity_id {
            // Expel players inside
            player_housing.expel_players_and_entities(
                ctx,
                crate::messages::action_request::ServerTeleportReason::PlayerHousingChangedLocation,
            );
            player_housing.entrance_building_entity_id = 0;
            PlayerHousingState::update_shared(
                ctx,
                player_housing,
                crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
            );
        }
    }
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_housing_evict_player(ctx: &ReducerContext, request: PlayerHousingEvictPlayerRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if actor_id == request.owner_entity_id {
        return Err("You cannot evict yourself".into());
    }

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let (player_housing, building) =
        PlayerHousingState::get_and_validate_player_housing(ctx, actor_id, request.building_entity_id, true, request.owner_entity_id)?;

    /*
    // New permission system
    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::CoOwner) {
        return Err("You don't have permission".into());
    }
    */

    // Check for claim ownership permissions
    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(building.claim_entity_id),
        "Building is not claimed"
    );

    let actor_claim_member = unwrap_or_err!(claim.get_member(ctx, actor_id), "Only claim members can evict");
    let target_claim_member_perms = if let Some(member) = claim.get_member(ctx, player_housing.entity_id) {
        claim.score_permissions(&member)
    } else {
        0
    };

    if claim.owner_player_entity_id != actor_id && !actor_claim_member.co_owner_permission {
        return Err("You need to be owner or co-owner to evict a player from this building".into());
    }

    if claim.score_permissions(&actor_claim_member) <= target_claim_member_perms {
        return Err("You can only evict players with fewer claim permissions".into());
    }

    // Evict.
    PlayerHousingEvictPlayerTimer::schedule(ctx, request.building_entity_id, request.owner_entity_id);

    Ok(())
}
