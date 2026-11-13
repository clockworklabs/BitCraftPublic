use bitcraft_macro::feature_gate;
use std::collections::HashMap;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        coordinates::{ChunkCoordinates, FloatHexTile, OffsetCoordinatesSmall, SmallHexTile},
        discovery::Discovery,
        game_state::{self},
        handlers::player::player_housing_change_entrance,
        reducer_helpers::interior_helpers,
    },
    messages::{action_request::ServerTeleportReason, components::*, static_data::player_housing_desc},
};

#[shared_table_reducer]
#[spacetimedb::reducer]
#[feature_gate]
fn player_housing_update(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    player_housing_update_impl(ctx, building_entity_id)
}

pub fn player_housing_update_impl(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let building = PlayerHousingState::get_and_validate_entrance_building(ctx, actor_id, building_entity_id)?;

    // possible to-do: deployables?

    let outside_dimension = ctx.db.location_state().entity_id().find(building_entity_id).unwrap().dimension;

    let mut highest_rank = 0;
    let mut template_building_id = 0;

    for player_housing in ctx.db.player_housing_desc().iter() {
        if Discovery::already_acquired_secondary(ctx, actor_id, player_housing.secondary_knowledge_id) && highest_rank < player_housing.rank
        {
            highest_rank = player_housing.rank;
            template_building_id = player_housing.template_building_id;
        }
    }

    if highest_rank == 0 {
        return Err("You do not own a personal house".into());
    }

    if let Some(mut player_housing) = ctx.db.player_housing_state().entity_id().find(actor_id) {
        if player_housing.rank != highest_rank {
            player_housing.expel_players_and_entities(ctx, ServerTeleportReason::PlayerHousingUnderMaintenance);

            let previous_network_id = player_housing.network_entity_id;
            let previous_exit_portal_id = player_housing.exit_portal_entity_id;
            // Create a BRAND NEW interior network in case we add new rooms or connections
            let network_entity_id = interior_helpers::create_player_interior(ctx, template_building_id, building_entity_id)?;

            // Update housing data
            player_housing.rank = highest_rank;
            player_housing.network_entity_id = network_entity_id;
            player_housing.exit_portal_entity_id = PlayerHousingState::find_portal_to_outside(ctx, network_entity_id, outside_dimension);

            // Transfer all furniture into matching dimensions
            transfer_dimension_contents(
                ctx,
                previous_network_id,
                previous_exit_portal_id,
                network_entity_id,
                player_housing.exit_portal_entity_id,
            );

            player_housing.copy_permissions(ctx)?;

            // Destroy old dimensions
            let portal = ctx
                .db
                .portal_state()
                .entity_id()
                .find(player_housing.exit_portal_entity_id)
                .unwrap();

            let oc = OffsetCoordinatesSmall {
                x: portal.destination_x,
                z: portal.destination_z,
                dimension: portal.destination_dimension,
            };
            interior_helpers::delete_dimension_network(ctx, previous_network_id, oc.into());
            PlayerHousingState::update_shared(
                ctx,
                player_housing,
                crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
            );
        } else if player_housing.entrance_building_entity_id == 0 {
            //Player was evicted
            return player_housing_change_entrance::reduce(ctx, building_entity_id, -1);
        }
    } else {
        PlayerHousingState::create_housing(ctx, actor_id, highest_rank, template_building_id, &building, outside_dimension)?;
    }

    Ok(())
}

fn transfer_dimension_contents(
    ctx: &ReducerContext,
    previous_network_entity: u64,
    previous_network_exit_portal: u64,
    new_network_entity: u64,
    new_network_exit_portal: u64,
) {
    let mut dimension_map: HashMap<u32, u32> = HashMap::new();
    let mut locations_map: HashMap<u64, u32> = HashMap::new();
    let mut mobiles_map: HashMap<u64, u32> = HashMap::new();
    let mut new_portals: Vec<(LocationState, u32)> = Vec::new();
    let mut previous_portals: Vec<(LocationState, u32)> = Vec::new();

    let mut previous_portals_entity_ids = Vec::new();

    // leaving the logs for now until we can test multi-dimensional transfer
    log::info!("previous_network_entity => {previous_network_entity} previous_network_exit_portal => {previous_network_exit_portal}");
    log::info!("new_network_entity => {new_network_entity} new_network_exit_portal => {new_network_exit_portal}");

    for dimension_description in ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(new_network_entity)
    {
        for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
            if let Some(portal) = ctx.db.portal_state().entity_id().find(location.entity_id) {
                new_portals.push((location, portal.destination_dimension));
                log::info!("new portal {} => dimension {}", location.entity_id, portal.destination_dimension);
            }
        }
    }

    for dimension_description in ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(previous_network_entity)
    {
        for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
            if let Some(portal) = ctx.db.portal_state().entity_id().find(location.entity_id) {
                // find the closest new portal matching this position
                log::info!(
                    "previous portal {} => dimension {}",
                    location.entity_id,
                    portal.destination_dimension
                );
                previous_portals.push((location, portal.destination_dimension));
                previous_portals_entity_ids.push(location.entity_id);
            } else {
                log::info!("previous entity {} => dimension {}", location.entity_id, location.dimension);
                locations_map.insert(location.entity_id, location.dimension);
            }
        }
        let chunk = ChunkCoordinates {
            x: 0,
            z: 0,
            dimension: dimension_description.dimension_id,
        };
        for mobile in ctx.db.mobile_entity_state().chunk_index().filter(chunk.chunk_index()) {
            if ctx.db.player_state().entity_id().find(mobile.entity_id).is_some() {
                // don't transfer players; they should be warped outside
            } else {
                mobiles_map.insert(mobile.entity_id, mobile.dimension);
            }
        }
    }

    // Keep track of all portals leading to another dimension that hasn't been mapped yet.
    // We will ignore those to find entrance portals for all dimensions (excluding the housing entrance portal)
    let mut entrance_portal_offsets: HashMap<u32, SmallHexTile> = HashMap::new();

    // Now link the portals to their matching counterparts in the new setup
    let start_dimension = ctx
        .db
        .location_state()
        .entity_id()
        .find(previous_network_exit_portal)
        .unwrap()
        .dimension;
    let target_dimension = ctx.db.location_state().entity_id().find(new_network_exit_portal).unwrap().dimension;

    dimension_map.insert(start_dimension, target_dimension);
    log::info!("[{start_dimension}] => [{target_dimension}]");

    let mut pending_dimensions = vec![start_dimension];
    while !pending_dimensions.is_empty() {
        let dimension = pending_dimensions.remove(0);
        let matching_dimension = *dimension_map.get(&dimension).unwrap();

        // the idea is that every previous portal has a matching portal in the newer, expended layout
        for previous_portal in previous_portals.iter().take(previous_portals.len() - 1) {
            if previous_portal.0.dimension == dimension {
                let mut check_loc = previous_portal.0.coordinates();
                check_loc.dimension = matching_dimension;
                let mut shortest_distance = i32::MAX;
                let mut shortest_index = 0;
                for (j, new_portal) in new_portals.iter().enumerate().take(new_portals.len() - 1) {
                    let distance = new_portal.0.coordinates().distance_to(check_loc);
                    if distance < shortest_distance {
                        shortest_distance = distance;
                        shortest_index = j;
                    }
                }

                let (new_portal_location, dim) = new_portals.remove(shortest_index);
                log::info!("[{}] => [{dim}]", previous_portal.1);
                if dimension_map.contains_key(&previous_portal.1) {
                    // this portal leads to an existing dimension, therefore it's an entrance portal for that room
                    entrance_portal_offsets.insert(
                        previous_portal.1,
                        new_portal_location.coordinates() - previous_portal.0.coordinates(),
                    );
                }

                dimension_map.insert(previous_portal.1, dim); // destination dimension from previous portal matches the one from new portal
                pending_dimensions.push(dim);
            }
        }
    }

    // Add player housing entrance to known locations
    let entrance_portal_offset = ctx
        .db
        .location_state()
        .entity_id()
        .find(new_network_exit_portal)
        .unwrap()
        .coordinates()
        - ctx
            .db
            .location_state()
            .entity_id()
            .find(previous_network_exit_portal)
            .unwrap()
            .coordinates();
    entrance_portal_offsets.insert(start_dimension, entrance_portal_offset);

    // Find all portals going to the entrance dimension
    let mut pending_dimension_entrances = vec![start_dimension];

    while !pending_dimension_entrances.is_empty() {
        // for each dimension, find the portal leading to it and add its dimension to pending dimensions
        let dim = pending_dimension_entrances.remove(0);
        for portal in previous_portals.iter().filter(|p| p.1 == dim) {
            pending_dimension_entrances.push(portal.0.dimension);
        }
    }

    // Finally... transfer non-portal locations to the new dimension
    for (location_entity_id, dimension) in locations_map {
        // don't transfer portal footprints
        if previous_portals_entity_ids.contains(&location_entity_id) {
            continue;
        }

        let location_delta = entrance_portal_offsets[&dimension];

        let mut location = ctx.db.location_state().entity_id().find(location_entity_id).unwrap();
        log::info!(
            "transfering {} to dimension {}",
            location_entity_id,
            dimension_map.get(&dimension).unwrap()
        );
        location.dimension = *dimension_map.get(&dimension).unwrap();
        let coord = location.coordinates() + location_delta;
        location.set_location(coord.into()); // update chunk index
        ctx.db.location_state().entity_id().update(location);
    }
    for (mobile_entity_id, dimension) in mobiles_map {
        let mut mobile_entity = ctx.db.mobile_entity_state().entity_id().find(mobile_entity_id).unwrap();
        mobile_entity.dimension = dimension;

        let location_delta = FloatHexTile::from(entrance_portal_offsets[&dimension]);
        let coord: FloatHexTile = location_delta + mobile_entity.coordinates_float();

        mobile_entity.set_location(coord.into()); // update chunk index
        ctx.db.mobile_entity_state().entity_id().update(mobile_entity);
    }
}
