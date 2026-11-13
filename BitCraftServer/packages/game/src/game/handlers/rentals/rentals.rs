use bitcraft_macro::feature_gate;
use std::time::Duration;

use crate::game::game_state;
use crate::game::reducer_helpers::timer_helpers::now_plus_secs;
use crate::messages::action_request::*;
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::*;
use crate::messages::static_data::AlertType;
use crate::{parameters_desc, unwrap_or_err};
use spacetimedb::{ReducerContext, Table};

fn validate_co_owner_permissions(ctx: &ReducerContext, actor_id: u64, rent_entity_id: u64) -> Result<(), String> {
    let rent = unwrap_or_err!(ctx.db.rent_state().entity_id().find(rent_entity_id), "Rental does not exist");
    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(rent.claim_entity_id),
        "Rental is not part of a claim"
    );
    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Not authorized".into());
    }
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_add_listing(ctx: &ReducerContext, request: RentAddListingRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    validate_co_owner_permissions(ctx, actor_id, request.rent_entity_id)?;

    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    if !rent.active {
        rent.active = true;
        ctx.db.rent_state().entity_id().update(rent);
    }
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_unlist(ctx: &ReducerContext, request: RentUnlistRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    validate_co_owner_permissions(ctx, actor_id, request.rent_entity_id)?;

    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    if rent.active {
        rent.active = false;
        ctx.db.rent_state().entity_id().update(rent);
    }
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_deposit_coins(ctx: &ReducerContext, request: RentDepositCoinsRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    if !rent.active {
        return Err("This rental is not listed yet".into());
    }

    if rent.eviction_timestamp.is_some() {
        return Err("Not authorized - you are being evicted.".into());
    }

    if !rent.is_tenant(actor_id) {
        return Err("Not a tenant of this rental".into());
    }

    rent.pay_rent(ctx, actor_id, request.amount)?;

    ctx.db.rent_state().entity_id().update(rent);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_remove_tenant(ctx: &ReducerContext, request: RentRemoveTenantRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    if !rent.active {
        return Err("This rental is not listed yet".into());
    }
    if !rent.is_tenant(actor_id) {
        return Err("Not authorized".into());
    }

    if let Some(idx) = rent.white_list.iter().position(|t| *t == request.tenant_entity_id) {
        if idx == 0 {
            return Err("Cannot remove renter".into());
        }
        rent.white_list.remove(idx);
        ctx.db.rent_state().entity_id().update(rent);
    } else {
        return Err("Not a tenant".into());
    }
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_add_tenant(ctx: &ReducerContext, request: RentAddTenantRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    if !rent.active {
        return Err("This rental is not listed yet".into());
    }

    if rent.eviction_timestamp.is_some() {
        return Err("Not authorized - you are being evicted.".into());
    }

    if !rent.is_tenant(actor_id) {
        return Err("Not authorized".into());
    }

    if rent.white_list.contains(&request.tenant_entity_id) {
        return Err("Already a tenant".into());
    }

    //DAB Note: hard-coded to 10 right now as per design. Might be a tech later on, if not we can set it a parameter.
    if rent.white_list.len() > 10 {
        return Err("Reached maximum amount of tenants".into());
    }

    rent.white_list.push(request.tenant_entity_id);
    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_set_daily_rate(ctx: &ReducerContext, request: RentSetDailyRateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    validate_co_owner_permissions(ctx, actor_id, request.rent_entity_id)?;
    let mut rent = ctx.db.rent_state().entity_id().find(request.rent_entity_id).unwrap();
    rent.daily_rent = request.daily_rate;
    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_purchase(ctx: &ReducerContext, request: RentPurchaseRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut rent = unwrap_or_err!(
        ctx.db.rent_state().entity_id().find(request.rent_entity_id),
        "Rental does not exist"
    );
    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(rent.claim_entity_id),
        "Rental is not part of a claim"
    );

    if !rent.white_list.is_empty() {
        return Err("Rental is not available".into());
    }

    if ctx
        .db
        .claim_member_state()
        .player_claim()
        .filter((actor_id, rent.claim_entity_id))
        .next()
        .is_none()
    {
        return Err("Not authorized".into());
    }

    if !rent.active && !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("This rental is not listed yet".into());
    }

    let amount = ctx.db.parameters_desc().version().find(0).unwrap().rent_deposit_days as u32 * rent.daily_rent;
    rent.pay_rent(ctx, actor_id, amount)?;
    rent.white_list.push(actor_id);
    rent.active = true;
    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_evict(ctx: &ReducerContext, request: RentEvictRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    // for now simply wipe the rent
    // DAB Note: need to add treasury concept to the claim
    let mut rent = unwrap_or_err!(
        ctx.db.rent_state().entity_id().find(request.rent_entity_id),
        "This rental no longer exists."
    );

    if !rent.active {
        return Err("This rental is not listed yet".into());
    }

    if rent.eviction_timestamp.is_some() {
        return Err("This tenant is already being evicted.".into());
    }

    let owning_claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(rent.claim_entity_id),
        "This rental is not linked to a claim"
    );

    if owning_claim.owner_player_entity_id != actor_id {
        return Err("Only the claim owner can evict players.".into());
    }

    if rent.defaulted {
        // A tenant defaulting on his rent has no right at all - instant eviction.
        AlertState::new(ctx, AlertType::EvictionStatement, rent.white_list[0], rent.entity_id)?;
        rent.clear();
    } else {
        let compensatory_fee =
            (rent.daily_rent as f32 * ctx.db.parameters_desc().version().find(0).unwrap().rent_eviction_compensation) as u32;
        let mut remaining_compensatory_fee = compensatory_fee;
        let mut claim_local = owning_claim.local_state(ctx);

        if claim_local.treasury > 0 {
            if claim_local.treasury < compensatory_fee {
                remaining_compensatory_fee -= claim_local.treasury;
                claim_local.treasury = 0;
            } else {
                claim_local.treasury -= compensatory_fee;
                remaining_compensatory_fee = 0;
            }
            ctx.db.claim_local_state().entity_id().update(claim_local);
        }

        if remaining_compensatory_fee > 0 {
            // Subtract from your inventory the remainder
            rent.pay_remaining_eviction_fee(ctx, actor_id, compensatory_fee, remaining_compensatory_fee)?;
        }

        let eviction_term: u64 = 24 * 60 * 60; // 1 day
        rent.eviction_timestamp = Some(ctx.timestamp + Duration::from_secs(eviction_term));
        // Start some eviction timer
        ctx.db
            .rent_evict_timer()
            .try_insert(RentEvictTimer {
                scheduled_id: 0,
                scheduled_at: now_plus_secs(eviction_term, ctx.timestamp),
                rent_entity_id: request.rent_entity_id,
            })
            .ok()
            .unwrap();

        // Alerts (24H)
        AlertState::new(ctx, AlertType::EvictionWarning, rent.white_list[0], rent.entity_id)?;
    }

    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::table(accessor = rent_evict_timer, scheduled(rent_evict_term, at = scheduled_at))]
pub struct RentEvictTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub rent_entity_id: u64,
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_evict_term(ctx: &ReducerContext, timer: RentEvictTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let mut rent = unwrap_or_err!(
        ctx.db.rent_state().entity_id().find(timer.rent_entity_id),
        "This rental no longer exists."
    );
    AlertState::new(ctx, AlertType::EvictionStatement, rent.white_list[0], rent.entity_id)?;
    rent.clear();
    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_terminate(ctx: &ReducerContext, request: RentTerminateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    // for now simply wipe the rent
    // DAB Note: need to add treasury concept to the claim
    let mut rent = unwrap_or_err!(
        ctx.db.rent_state().entity_id().find(request.rent_entity_id),
        "This rental no longer exists."
    );

    if !rent.active {
        return Err("This rental is not listed yet".into());
    }

    if rent.eviction_timestamp.is_some() {
        return Err("Not authorized - you are being evicted.".into());
    }

    if !rent.white_list.is_empty() && rent.white_list[0] != actor_id {
        return Err("Only the renter can terminate the rental.".into());
    }

    rent.collect_paid_rent(ctx, actor_id)?;
    rent.clear();

    ctx.db.rent_state().entity_id().update(rent);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn rent_collect_eviction_fee(ctx: &ReducerContext, rent_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut rent = ctx.db.rent_state().entity_id().find(rent_entity_id).unwrap();
    if !rent.active {
        return Err("This rental is not listed yet".into());
    }

    if rent.eviction_timestamp.is_none() {
        return Err("You are not being evicted.".into());
    }

    if !rent.is_renter(actor_id) {
        return Err("Not the owner of this rental".into());
    }

    // Dismiss any pending Eviction Warning from the renter (if the alert wasn't dismissed by the user)
    AlertState::delete(ctx, AlertType::EvictionWarning, actor_id, rent_entity_id);

    rent.collect_paid_rent(ctx, actor_id)?;
    rent.clear();

    ctx.db.rent_state().entity_id().update(rent);

    Ok(())
}
