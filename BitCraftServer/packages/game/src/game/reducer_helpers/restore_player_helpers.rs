use spacetimedb::{log, ReducerContext, Table};

use crate::{
    deployable_state_v2, dimension_description_state,
    game::{dimensions, entities::location::MobileEntityState, reducer_helpers::dimension_helpers::clamp_within_dimension_bounds},
    location_cache,
    messages::components::{claim_state, PlayerState},
    mobile_entity_state, mounting_state, player_state, OffsetCoordinatesFloat, OffsetCoordinatesSmall,
};

use super::dimension_helpers::{get_dimension_bounds, is_within_dimension_bounds};

//DAB Note: This is a temporary hack for 5.0
pub fn auto_unstuck_player_and_deployable(ctx: &ReducerContext, player_entity_id: &u64) {
    let player_mobile_entity_state = ctx.db.mobile_entity_state().entity_id().find(player_entity_id).unwrap();
    let player_unstuck_location = get_player_unstuck_location(ctx, &player_mobile_entity_state);

    if let Some(mounting_state) = ctx.db.mounting_state().entity_id().find(player_entity_id) {
        if let Some(deployable_state) = ctx.db.deployable_state_v2().entity_id().find(mounting_state.deployable_entity_id) {
            if deployable_state.owner_id == *player_entity_id {
                let mut deployable_location = ctx
                    .db
                    .mobile_entity_state()
                    .entity_id()
                    .find(deployable_state.entity_id)
                    .unwrap()
                    .offset_coordinates_float();
                let bounds = get_dimension_bounds(ctx, &player_mobile_entity_state.dimension);

                if !is_within_dimension_bounds(&deployable_location, bounds) {
                    log::info!("Auto-unstucking deployable with id: {}", deployable_state.entity_id);

                    if let Some(player_unstuck_location) = player_unstuck_location {
                        unstuck_entity(ctx, &deployable_state.entity_id, &player_unstuck_location);
                    } else {
                        clamp_within_dimension_bounds(&mut deployable_location, bounds);
                        unstuck_entity(ctx, &deployable_state.entity_id, &deployable_location);
                    }
                }
            } else {
                //Player is a passenger - dismount deployable
                ctx.db.mounting_state().entity_id().delete(mounting_state.entity_id);
                PlayerState::collect_stats(ctx, *player_entity_id);
            }
        } else {
            log::error!(
                "Player with entity_id {} has a mounting state but deployable is missing. Deleting mounting state",
                player_entity_id
            );
            ctx.db.mounting_state().entity_id().delete(player_entity_id);
        }
    }

    if let Some(player_unstuck_location) = player_unstuck_location {
        log::info!("Auto-unstucking player with id: {}", player_entity_id);

        unstuck_entity(ctx, player_entity_id, &player_unstuck_location);
    }
}

fn get_player_unstuck_location(ctx: &ReducerContext, player_mobile_entity_state: &MobileEntityState) -> Option<OffsetCoordinatesFloat> {
    // Try to teleport within bounds
    if player_mobile_entity_state.dimension == dimensions::OVERWORLD
        || ctx
            .db
            .dimension_description_state()
            .dimension_id()
            .find(player_mobile_entity_state.dimension)
            .is_some()
    {
        let mut player_location = player_mobile_entity_state.offset_coordinates_float();
        let bounds = get_dimension_bounds(ctx, &player_mobile_entity_state.dimension);

        if is_within_dimension_bounds(&player_location, bounds) {
            return None;
        }

        clamp_within_dimension_bounds(&mut player_location, bounds);
        return Some(player_location);
    }

    // Try to teleport home
    let player = ctx
        .db
        .player_state()
        .entity_id()
        .find(player_mobile_entity_state.entity_id)
        .unwrap();
    let home_location = player.teleport_location.location;
    if ctx
        .db
        .dimension_description_state()
        .dimension_id()
        .find(home_location.dimension)
        .is_some()
    {
        return Some(OffsetCoordinatesFloat::from(home_location));
    }

    // Try to teleport to one of the player's claims
    for claim_description_state in ctx.db.claim_state().iter() {
        if let Some(claim_location) = claim_description_state.local_state(ctx).location {
            if claim_description_state
                .get_member(ctx, player_mobile_entity_state.entity_id)
                .is_some()
            {
                return Some(OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(claim_location)));
            }
        }
    }

    // Use the first spawn location as a fallback
    let location_cache = ctx.db.location_cache().version().find(0).unwrap();
    let spawn_location = location_cache.spawn_locations.first().unwrap();

    Some(OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(spawn_location)))
}

fn unstuck_entity(ctx: &ReducerContext, entity_id: &u64, offset_coordinates: &OffsetCoordinatesFloat) {
    ctx.db
        .mobile_entity_state()
        .entity_id()
        .update(MobileEntityState::for_location(*entity_id, *offset_coordinates, ctx.timestamp));
}
