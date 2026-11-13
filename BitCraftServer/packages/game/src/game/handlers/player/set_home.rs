use bitcraft_macro::feature_gate;
use std::time::Duration;

use entities::resource_clump::SmallHexTile;
use spacetimedb::{ReducerContext, Table};

use crate::building_desc;
use crate::game::coordinates::FloatHexTile;
use crate::game::game_state::game_state_filters::coordinates_float;
use crate::messages::static_data::FootprintType;
use crate::{
    game::{
        entities,
        game_state::{self, game_state_filters},
    },
    messages::{action_request::PlayerSetHomeRequest, components::*, events::*, static_data::BuildingCategory},
    unwrap_or_err,
};

pub fn event_delay(_actor_id: u64, _request: &PlayerSetHomeRequest) -> Duration {
    Duration::from_secs_f32(1.0)// wait 1 second to do the set home animation
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn set_home(ctx: &ReducerContext, request: PlayerSetHomeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let result = reduce(ctx, actor_id, request.target_entity_id);
    if result.is_ok() {
        ctx.db.set_home_event().insert(SetHomeEvent { actor_id, request });
    }
    result
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, target_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let mut player_state = unwrap_or_err!(ctx.db.player_state().entity_id().find(actor_id), "Invalid player");

    let building = get_target_home_building(ctx, actor_id, target_entity_id)?;

    let location = game_state_filters::offset_coordinates(ctx, building.entity_id);
    player_state.teleport_location = TeleportLocation {
        location,
        location_type: TeleportLocationType::HomeLocation,
    };
    ctx.db.player_state().entity_id().update(player_state);

    PlayerActionState::success(
        ctx,
        actor_id,
        PlayerActionType::SetHome,
        PlayerActionType::SetHome.get_layer(ctx),
        0,
        None,
        None,
        game_state::unix_ms(ctx.timestamp),
    );

    Ok(())
}

pub fn get_target_home_building(ctx: &ReducerContext, actor_id: u64, target_entity_id: u64) -> Result<BuildingState, String> {
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(target_entity_id),
        "Building doesn't exist"
    );

    let building_description = unwrap_or_err!(
        ctx.db.building_desc().id().find(building.building_description_id),
        "Building doesn't exist in the static data"
    );

    let coordinates = game_state_filters::coordinates(ctx, target_entity_id);
    let actor_coords: FloatHexTile = coordinates_float(ctx, actor_id);
    let mut target_coords = SmallHexTile::from(coordinates);

    // if building has a footprint, find the closest footprint tile
    let footprint = building_description.get_footprint(&coordinates, building.direction_index);
    if !footprint.is_empty() {
        let mut min_dist = actor_coords.distance_to(target_coords.into());
        for (coord, footprint_type) in footprint {
            if footprint_type != FootprintType::Perimeter {
                let dist = actor_coords.distance_to(coord.into());
                if dist < min_dist {
                    target_coords = coord;
                    min_dist = dist;
                }
            }
        }
    }

    if actor_coords.distance_to(target_coords.into()) > 2.0 {
        return Err("You are too far away.".into());
    }

    if !building_description.has_category(ctx, BuildingCategory::Residential) {
        return Err("You cannot set home there".into());
    }
    Ok(building)
}
