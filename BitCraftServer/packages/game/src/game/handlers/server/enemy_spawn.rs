use crate::game::dimensions;
use crate::game::handlers::authentication::has_role;
use crate::messages::action_request::EnemySpawnRequest;
use crate::messages::authentication::Role;
use crate::messages::components::{ClaimState, EnemyState};
use crate::messages::static_data::EnemyType;
use crate::{claim_state, enemy_desc, herd_state, unwrap_or_return};
use crate::{game::claim_helper, resource_state};

use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn enemy_spawn(ctx: &ReducerContext, request: EnemySpawnRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    reduce(ctx, request);

    Ok(())
}

#[spacetimedb::reducer]
pub fn enemy_spawn_batch(ctx: &ReducerContext, requests: Vec<EnemySpawnRequest>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    for request in requests {
        reduce(ctx, request);
    }

    Ok(())
}

fn reduce(ctx: &ReducerContext, request: EnemySpawnRequest) {
    let attached_resource = ctx.db.resource_state().entity_id().find(request.herd_entity_id);

    // Don't spawn over player claims, but don't return an error either. Just ignore this spawn
    // (except for resource spawns, those are rude and inconsiderate).
    // Note: we might want a more general way to check herd type if we start attaching herds to buildings or other type of entities.
    // Note: we will allow spawns on claimed tiles if they are in an interior.
    if should_suppress_overworld_herd_spawn(
        attached_resource.is_some(),
        request.coord.dimension,
        claim_helper::get_claim_on_tile(ctx, request.coord.into())
            .and_then(|claim_tile| ctx.db.claim_state().entity_id().find(claim_tile.claim_id)),
    ) {
        return;
    }

    let enemy_type: EnemyType = request.enemy_type;
    let mut herd = unwrap_or_return!(
        ctx.db.herd_state().entity_id().find(request.herd_entity_id),
        "Request a spawn from an unexistent herd"
    );

    let enemy_state = EnemyState::new(ctx, enemy_type, herd.entity_id);
    let offset = request.coord;
    let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type as i32).unwrap();
    unwrap_or_return!(
        EnemyState::spawn_enemy(ctx, &enemy_desc, enemy_state, offset, Some(&herd)).ok(),
        "Failed to spawn enemy"
    );
    herd.current_population += 1;
    herd.ignore_eagerness = false; // no longer need to spawn everything at once

    ctx.db.herd_state().entity_id().update(herd);
}

fn should_suppress_overworld_herd_spawn(attached_to_resource: bool, dimension_id: u32, claim: Option<ClaimState>) -> bool {
    if attached_to_resource || dimension_id != dimensions::OVERWORLD {
        return false;
    }

    claim.is_some_and(|claim_state| !claim_state.neutral)
}
