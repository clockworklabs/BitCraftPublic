use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{
        action_request::TargetUpdateRequest,
        authentication::Role,
        components::{health_state, TargetState},
    },
    target_state, PlayerTimestampState, ThreatState,
};

use crate::game::handlers::authentication::has_role;

#[spacetimedb::reducer]
#[feature_gate("combat")]
pub fn target_update(ctx: &ReducerContext, request: TargetUpdateRequest) -> Result<(), String> {
    if let Ok(actor_id) = game_state::actor_id(ctx, false) {
        if actor_id != request.owner_entity_id {
            return Err("Unauthorized".into());
        }
        game_state::ensure_signed_in(ctx, actor_id)?;
        PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    } else {
        if !has_role(ctx, &ctx.sender(), Role::Admin) {
            return Err("Invalid permissions".into());
        }
    }
    reduce(ctx, request.owner_entity_id, request.target_entity_id, request.generate_aggro)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, target_entity_id: u64, generate_aggro: bool) -> Result<(), String> {
    if let Some(health) = ctx.db.health_state().entity_id().find(target_entity_id) {
        if health.health == 0.0 {
            // note: infinite health buildings have -1 hp, therefore we can't check for <= 0.0; besides the health_state hp functions prevents a negative value outcome
            return Err("~This target is no longer valid".into());
        }
    }

    if generate_aggro {
        ThreatState::add_threat(ctx, actor_id, target_entity_id, 0.0);
        ThreatState::add_threat(ctx, target_entity_id, actor_id, 0.0);
    }

    if let Some(target_state) = ctx.db.target_state().entity_id().find(actor_id) {
        // Not a target change
        if target_entity_id == target_state.target_entity_id {
            return Ok(());
        }
        // delete or update existing target
        if target_entity_id == 0 {
            ctx.db.target_state().entity_id().delete(actor_id);
        } else {
            let mut target_state = target_state.clone();
            target_state.target_entity_id = target_entity_id;
            ctx.db.target_state().entity_id().update(target_state);
        }
    } else {
        // insert new target
        if target_entity_id != 0 {
            let target_state = TargetState {
                entity_id: actor_id,
                target_entity_id,
            };
            if ctx.db.target_state().try_insert(target_state).is_err() {
                return Err("Unable to insert target".into());
            }
        }
    }
    Ok(())
}
