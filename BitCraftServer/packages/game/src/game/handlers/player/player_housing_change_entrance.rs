use bitcraft_macro::feature_gate;
use std::time::Duration;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table, TimeDuration};

use crate::{
    game::game_state::{self},
    inter_module,
    messages::{
        action_request::ServerTeleportReason,
        authentication::ServerIdentity,
        components::*,
        generic::world_region_state,
        static_data::{building_desc, BuildingFunction},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
fn update_moving_cost(ctx: &ReducerContext, player_housing_entity_id: u64, moving_cost: i32) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;
    let mut player_housing_cost = ctx
        .db
        .player_housing_moving_cost_state()
        .entity_id()
        .find(player_housing_entity_id)
        .unwrap();
    player_housing_cost.moving_time_cost_minutes = moving_cost;
    ctx.db.player_housing_moving_cost_state().entity_id().update(player_housing_cost);
    Ok(())
}

#[shared_table_reducer]
#[spacetimedb::reducer]
#[feature_gate]
pub fn player_housing_change_entrance(ctx: &ReducerContext, building_entity_id: u64, expected_time_cost: i32) -> Result<(), String> {
    reduce(ctx, building_entity_id, expected_time_cost)
}

pub fn reduce(ctx: &ReducerContext, building_entity_id: u64, expected_time_cost: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let (mut player_housing, building) =
        PlayerHousingState::get_and_validate_player_housing(ctx, actor_id, building_entity_id, false, actor_id)?;

    let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();

    let housing_slots = BuildingFunction::max_housing_slots(&building_desc) as usize;
    if housing_slots == 0 {
        return Err("This building can't house players".into());
    }
    if housing_slots > 0 {
        let current_houses = ctx
            .db
            .player_housing_state()
            .entrance_building_entity_id()
            .filter(building_entity_id)
            .count();
        if current_houses >= housing_slots {
            return Err("This building cannot house anymore players".into());
        }
    }

    // Off-region houses have different rules, we will check off-region if it's still empty or not
    let region = ctx.db.world_region_state().iter().next().unwrap();
    if player_housing.region_index != region.region_index {
        return inter_module::transfer_player_housing::send_message(ctx, actor_id, building.entity_id);
    }

    // Same region, same claim means instant transfer
    let mut same_claim = false;
    if let Some(old_building) = ctx.db.building_state().entity_id().find(player_housing.entrance_building_entity_id) {
        same_claim = old_building.claim_entity_id == building.claim_entity_id;
    }

    let was_empty = player_housing.is_empty;
    player_housing.update_is_empty_flag_self(ctx);

    if !same_claim {
        let minutes_to_move = ctx
            .db
            .player_housing_moving_cost_state()
            .entity_id()
            .find(player_housing.entity_id)
            .unwrap()
            .moving_time_cost_minutes;
        if player_housing.is_empty != was_empty || (expected_time_cost != minutes_to_move && expected_time_cost >= 0) {
            spacetimedb::volatile_nonatomic_schedule_immediate!(update_moving_cost(player_housing.entity_id, minutes_to_move));
            return Err("Housing contents changed, move was cancelled.".into());
        }

        let end_of_move = ctx.timestamp + TimeDuration::from_duration(Duration::from_secs(minutes_to_move as u64 * 60));
        player_housing.locked_until = end_of_move;
    }

    player_housing.expel_players_and_entities(ctx, ServerTeleportReason::PlayerHousingChangedLocation);

    player_housing.entrance_building_entity_id = building.entity_id;

    let mut portal_state = unwrap_or_err!(
        ctx.db.portal_state().entity_id().find(player_housing.exit_portal_entity_id),
        "Your house has no exit portal."
    );

    portal_state.target_building_entity_id = building_entity_id;
    update_portal_position(ctx, portal_state)?;

    PlayerHousingState::update_shared(ctx, player_housing, inter_module::InterModuleDestination::GlobalAndAllOtherRegions);

    Ok(())
}

pub fn update_portal_position(ctx: &ReducerContext, mut portal_state: PortalState) -> Result<(), String> {
    let location = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(portal_state.target_building_entity_id),
        "Location not found for player house exit portal."
    );

    let oc = location.offset_coordinates();
    portal_state.destination_x = oc.x;
    portal_state.destination_z = oc.z;
    portal_state.destination_dimension = oc.dimension;
    ctx.db.portal_state().entity_id().update(portal_state);

    Ok(())
}
