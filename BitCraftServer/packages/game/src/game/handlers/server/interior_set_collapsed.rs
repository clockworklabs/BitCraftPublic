use spacetimedb::{ReducerContext, Table};

use crate::{
    building_state, dimension_description_state, dimension_network_state,
    game::{
        game_state::{self, game_state_filters},
        handlers::server::server_teleport_player::TeleportPlayerTimer,
        reducer_helpers::{building_helpers::delete_building, interior_helpers, timer_helpers::now_plus_secs},
    },
    interior_network_desc,
    messages::{
        action_request::ServerTeleportReason,
        authentication::ServerIdentity,
        components::{DimensionNetworkState, InteriorPlayerCountState, PortalState},
    },
    player_state, portal_state, unwrap_or_err,
};

use super::server_teleport_player::teleport_player_timer;

#[spacetimedb::table(name = interior_set_collapsed_timer, scheduled(interior_set_collapsed_scheduled, at = scheduled_at))]
pub struct InteriorSetCollapsedTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub dimension_network_entity_id: u64,
    pub is_collapsed: bool,
}

#[spacetimedb::reducer]
pub fn interior_set_collapsed_scheduled(ctx: &ReducerContext, timer: InteriorSetCollapsedTimer) -> Result<(), String> {
    interior_set_collapsed(ctx, timer.dimension_network_entity_id, timer.is_collapsed)
}

#[spacetimedb::reducer]
pub fn interior_set_collapsed(ctx: &ReducerContext, dimension_network_entity_id: u64, is_collapsed: bool) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(&ctx)?;

    let mut dimension_network = unwrap_or_err!(
        ctx.db.dimension_network_state().entity_id().find(&dimension_network_entity_id),
        "Invalid dimension network"
    )
    .clone();

    //Disable all portals leading into entrance dimension (make interior unenterable)
    let portals: Vec<PortalState> = ctx
        .db
        .portal_state()
        .destination_dimension()
        .filter(dimension_network.entrance_dimension_id)
        .collect();
    for mut portal in portals {
        portal.enabled = !is_collapsed;
        ctx.db.portal_state().entity_id().update(portal);
    }

    dimension_network.is_collapsed = is_collapsed;
    if is_collapsed {
        let building = ctx.db.building_state().entity_id().find(&dimension_network.building_id).unwrap();
        let interior_descriptor = ctx
            .db
            .interior_network_desc()
            .building_id()
            .find(building.building_description_id)
            .unwrap();
        dimension_network.collapse_respawn_timestamp =
            game_state::unix_ms(ctx.timestamp) + (interior_descriptor.respawn_time as u64 * 1000);
        incapacitate_and_teleport_players(ctx, &dimension_network);

        ctx.db
            .interior_set_collapsed_timer()
            .try_insert(InteriorSetCollapsedTimer {
                scheduled_id: 0,
                scheduled_at: now_plus_secs(interior_descriptor.respawn_time as u64, ctx.timestamp),
                dimension_network_entity_id: dimension_network_entity_id,
                is_collapsed: false,
            })
            .ok()
            .unwrap();

        if interior_descriptor.destroy_building_on_collapse {
            delete_building(ctx, 0, building.entity_id, None, true, false);
        }
    } else {
        dimension_network.collapse_respawn_timestamp = 0;
        interior_helpers::respawn_interior(ctx, dimension_network_entity_id);
    }
    ctx.db.dimension_network_state().entity_id().update(dimension_network);
    InteriorPlayerCountState::reset(ctx, dimension_network_entity_id);

    spacetimedb::log::info!("Interior set collapsed {}", is_collapsed);

    Ok(())
}

fn incapacitate_and_teleport_players(ctx: &ReducerContext, dimension_network: &DimensionNetworkState) {
    let teleport_oc_float = interior_helpers::find_teleport_coordinates_for_interior_destruction(ctx, dimension_network.building_id);

    let dimensions: Vec<u32> = ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(dimension_network.entity_id)
        .map(|a| a.dimension_id)
        .collect();
    let players: Vec<u64> = ctx
        .db
        .player_state()
        .iter()
        .filter(|a| dimensions.contains(&game_state_filters::coordinates_float(ctx, a.entity_id).dimension))
        .map(|a| a.entity_id)
        .collect();

    //This could've been a function call except we need to somehow pass teleport reason to players
    for player in players {
        ctx.db
            .teleport_player_timer()
            .try_insert(TeleportPlayerTimer {
                scheduled_at: ctx.timestamp.into(),
                scheduled_id: 0,
                player_entity_id: player,
                location: teleport_oc_float,
                reason: ServerTeleportReason::RuinCollapse,
            })
            .ok()
            .unwrap();
    }
}
