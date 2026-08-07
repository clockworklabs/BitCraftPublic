use std::time::Duration;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    agents,
    game::{autogen::_delete_entity::delete_entity, handlers::authentication::has_role},
    messages::{
        authentication::{Role, ServerIdentity},
        components::{
            crumb_trail_contribution_lock_state, crumb_trail_contribution_spent_state, crumb_trail_state, prospecting_state,
            resource_state, signed_in_player_state, CrumbTrailState, ResourceState,
        },
    },
};

#[spacetimedb::table(name = crumb_tail_cleanup_timer, scheduled(crumb_tail_cleanup_agent_loop, at = scheduled_at))]
pub struct CrumbTrailCleanupTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
}

const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;

pub fn update_timer(ctx: &ReducerContext) {
    let tick_length = SECONDS_IN_A_DAY;
    let mut count = 0;
    for mut timer in ctx.db.crumb_tail_cleanup_timer().iter() {
        count += 1;
        timer.scheduled_at = Duration::from_secs(tick_length).into();
        ctx.db.crumb_tail_cleanup_timer().scheduled_id().update(timer);
        log::info!("crumb_tail cleanup agent timer was updated");
    }
    if count > 1 {
        log::error!("More than one crumb_tail cleanup agents running!");
    }
}

pub fn init(ctx: &ReducerContext) {
    let tick_length = 60 * 10; // 10 minutes
    ctx.db
        .crumb_tail_cleanup_timer()
        .try_insert(CrumbTrailCleanupTimer {
            scheduled_id: 0,
            scheduled_at: Duration::from_secs(tick_length).into(),
        })
        .ok()
        .unwrap();
}

#[spacetimedb::reducer]
fn crumb_tail_cleanup_agent_insert(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Invalid permissions".into());
    }
    init(ctx);
    Ok(())
}

#[spacetimedb::reducer]
fn crumb_tail_cleanup_agent_loop(ctx: &ReducerContext, _timer: CrumbTrailCleanupTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        log::error!("Unauthorized access to crumb_tail cleanup agent");
        return;
    }

    if !agents::should_run(ctx) {
        return;
    }

    let mut count = 0;
    for mut crumb_trail in ctx.db.crumb_trail_state().iter() {
        let mut active = false;
        for prospecting in ctx.db.prospecting_state().crumb_trail_entity_id().filter(crumb_trail.entity_id) {
            // If nobody signed in is following this trail add a strike counter for clean-up
            if ctx.db.signed_in_player_state().entity_id().find(prospecting.entity_id).is_some() {
                active = true;
                break;
            }
        }
        if active {
            // someone is active, reset clean up strikes
            if crumb_trail.clean_up_counter > 0 {
                crumb_trail.clean_up_counter = 0;
                ctx.db.crumb_trail_state().entity_id().update(crumb_trail);
            }
        } else {
            crumb_trail.clean_up_counter += 1;
            if crumb_trail.clean_up_counter >= 3 {
                delete_crumb_trail(ctx, &crumb_trail);
                count += 1;
            } else {
                ctx.db.crumb_trail_state().entity_id().update(crumb_trail);
            }
        }
    }

    if count > 0 {
        log::info!("CrumbTrail Cleanup Agent deleted {count} trails");
    }
}

pub fn delete_crumb_trail(ctx: &ReducerContext, crumb_trail: &CrumbTrailState) {
    ctx.db.prospecting_state().crumb_trail_entity_id().delete(crumb_trail.entity_id);
    ctx.db
        .crumb_trail_contribution_spent_state()
        .crumb_trail_entity_id()
        .delete(crumb_trail.entity_id);
    ctx.db
        .crumb_trail_contribution_lock_state()
        .crumb_trail_entity_id()
        .delete(crumb_trail.entity_id);
    ctx.db.crumb_trail_state().entity_id().delete(crumb_trail.entity_id);

    // despawn prizes
    for prize in &crumb_trail.prize_entity_ids {
        if let Some(resource) = ctx.db.resource_state().entity_id().find(prize) {
            ResourceState::despawn(ctx, *prize, resource.resource_id);
        } else {
            // despawn herd... or non-resource prize entity
            delete_entity(ctx, *prize);
        }
    }
}

pub fn clean_up_if_final_prize_resource_deleted(ctx: &ReducerContext, prize_entity_id: u64) {
    let Some(contribution_lock) = ctx.db.crumb_trail_contribution_lock_state().entity_id().find(&prize_entity_id) else {
        return;
    };

    let Some(crumb_trail) = ctx
        .db
        .crumb_trail_state()
        .entity_id()
        .find(&contribution_lock.crumb_trail_entity_id)
    else {
        return;
    };

    if !crumb_trail
        .prize_entity_ids
        .iter()
        .any(|entity_id| ctx.db.resource_state().entity_id().find(entity_id).is_some())
    {
        delete_crumb_trail(ctx, &crumb_trail);
    }
}
