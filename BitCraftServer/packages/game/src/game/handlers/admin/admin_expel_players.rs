use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        coordinates::{
            hex_coordinates::HexCoordinates, offset_coordinates::OffsetCoordinates, region_coordinates::RegionCoordinates, FloatHexTile,
            LargeHexTile, SmallHexTile,
        },
        dimensions,
        handlers::authentication::has_role,
        reducer_helpers::{building_helpers::delete_building, deployable_helpers},
    },
    inter_module::{system_chat_broadcast::sytem_chat_broadcast_timer, transfer_player},
    messages::{
        authentication::Role,
        components::*,
        empire_shared::{empire_node_siege_state, empire_settlement_state, EmpireNodeSiegeState, EmpireSettlementState},
        generic::{region_control_info, world_region_state},
        static_data::{deployable_desc, DeployableType},
        util::OffsetCoordinatesSmallMessage,
    },
    unwrap_or_err,
    utils::from_ctx::FromCtx,
    PlaceableState, PlayerHousingState, WorldRegionState,
};

const WATCHTOWER_BUILDING_DESC_ID: i32 = 90_000;
const RUINED_WAYSTONE_BUILDING_DESC_ID: i32 = 359_099_015;
const TELEPORT_OFFSET_FROM_REGION_EDGE: i32 = 2;

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_expel_players(ctx: &ReducerContext, commit: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let region = unwrap_or_err!(ctx.db.world_region_state().id().find(0), "World region state not found");
    let control = unwrap_or_err!(
        ctx.db.region_control_info().region_id().find(region.region_index),
        "Region control state not found"
    );
    if control.allow_players {
        return Err("Close the region before running admin_expel_players".into());
    }

    let houses: Vec<PlayerHousingState> = ctx
        .db
        .player_housing_state()
        .iter()
        .filter(|house| house.region_index == region.region_index)
        .collect();
    let placeables: Vec<PlaceableState> = ctx.db.placeable_state().iter().collect();
    let watchtower_ids = watchtower_ids_in_region(ctx, &region);
    let empire_settlements = empire_settlements_in_region(ctx, &region);
    let ruined_waystones = ruined_waystones(ctx);

    let player_destinations: Vec<(u64, FloatHexTile, Option<SmallHexTile>)> = ctx
        .db
        .player_state()
        .iter()
        .filter_map(|player| {
            player_overworld_location(ctx, player.entity_id).map(|location| {
                (
                    player.entity_id,
                    location,
                    player_home_is_in_region(ctx, player.teleport_location.location, &region),
                )
            })
        })
        .map(|(player_id, location, home_is_in_region)| {
            let destination = closest_open_neighbor_destination(ctx, &region, location)?;
            let home_location = home_is_in_region.then(|| nearest_ruined_waystone(ctx, &region, destination, &ruined_waystones)).flatten();
            Ok((player_id, destination, home_location))
        })
        .collect::<Result<_, String>>()?;
    let deployables_to_store = deployed_player_deployables(ctx, &player_destinations);

    log::info!(
        "admin_expel_players {} region {}: {} placeables, {} houses, {} watchtowers, {} empire_settlements, {} scheduled broadcasts, {} players, {} homes reassigned, {} deployables to store",
        if commit { "COMMIT" } else { "DRY RUN" },
        region.region_index,
        placeables.len(),
        houses.len(),
        watchtower_ids.len(),
        empire_settlements.len(),
        ctx.db.sytem_chat_broadcast_timer().count(),
        player_destinations.len(),
        player_destinations.iter().filter(|(_, _, home_location)| home_location.is_some()).count(),
        deployables_to_store.len(),
    );
    if !commit {
        return Ok(());
    }

    for timer in ctx.db.sytem_chat_broadcast_timer().iter() {
        ctx.db.sytem_chat_broadcast_timer().scheduled_id().delete(timer.scheduled_id);
    }
    for placeable in placeables {
        placeable.despawn(ctx);
    }
    for house in houses {
        PlayerHousingState::delete_shared(ctx, house, crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions);
    }
    for watchtower_id in watchtower_ids {
        delete_watchtower(ctx, watchtower_id);
    }
    for settlement in empire_settlements {
        EmpireSettlementState::delete_shared(
            ctx,
            settlement,
            crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
        );
    }
    for deployable in deployables_to_store {
        deployable_helpers::store_deployable(ctx, deployable.owner_id, deployable.entity_id, false)?;
    }
    for (player_id, destination, home_location) in player_destinations {
        if let Some(home_location) = home_location {
            let mut player_state = ctx.db.player_state().entity_id().find(&player_id).unwrap();
            player_state.teleport_location.location = home_location.into();
            player_state.teleport_location.location_type = TeleportLocationType::HomeLocation;
            ctx.db.player_state().entity_id().update(player_state);
        }
        transfer_player::send_message(ctx, player_id, destination, true, 0.0)?;
    }
    Ok(())
}

fn delete_watchtower(ctx: &ReducerContext, watchtower_entity_id: u64) {
    let sieges: Vec<EmpireNodeSiegeState> = ctx
        .db
        .empire_node_siege_state()
        .building_entity_id()
        .filter(watchtower_entity_id)
        .collect();
    for siege in sieges {
        EmpireNodeSiegeState::delete_shared(ctx, siege, crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions);
    }

    delete_building(ctx, 0, watchtower_entity_id, None, false, false);
}

fn empire_settlements_in_region(ctx: &ReducerContext, region: &WorldRegionState) -> Vec<EmpireSettlementState> {
    ctx.db
        .empire_settlement_state()
        .iter()
        .filter(|settlement| {
            RegionCoordinates::from_ctx(ctx, SmallHexTile::from(settlement.location)).to_region_index(region.region_count_sqrt)
                == region.region_index
        })
        .collect()
}

fn deployed_player_deployables(ctx: &ReducerContext, player_destinations: &[(u64, FloatHexTile, Option<SmallHexTile>)]) -> Vec<DeployableStateV2> {
    let mut deployables_to_store = Vec::new();

    for (player_id, _, _) in player_destinations {
        for deployable in ctx.db.deployable_state_v2().owner_id().filter(*player_id) {
            if ctx.db.mobile_entity_state().entity_id().find(deployable.entity_id).is_none() {
                continue;
            }

            let description = ctx.db.deployable_desc().id().find(deployable.deployable_description_id).unwrap();
            if description.deployable_type != DeployableType::SiegeEngine {
                deployables_to_store.push(deployable);
            }
        }
    }

    deployables_to_store
}

fn player_home_is_in_region(
    ctx: &ReducerContext,
    home_location: OffsetCoordinatesSmallMessage,
    region: &WorldRegionState,
) -> bool {
    let home_location = SmallHexTile::from(home_location);
    home_location.dimension == dimensions::OVERWORLD
        && RegionCoordinates::from_ctx(ctx, home_location).to_region_index(region.region_count_sqrt) == region.region_index
}

fn ruined_waystones(ctx: &ReducerContext) -> Vec<(u64, SmallHexTile)> {
    ctx.db
        .building_state()
        .building_description_id()
        .filter(RUINED_WAYSTONE_BUILDING_DESC_ID)
        .filter_map(|building| {
            ctx.db
                .location_state()
                .entity_id()
                .find(&building.entity_id)
                .map(|location| (building.entity_id, location.coordinates()))
        })
        .collect()
}

fn nearest_ruined_waystone(
    ctx: &ReducerContext,
    region: &WorldRegionState,
    destination: FloatHexTile,
    ruined_waystones: &[(u64, SmallHexTile)],
) -> Option<SmallHexTile> {
    let destination_region = RegionCoordinates::from_ctx(ctx, destination).to_region_index(region.region_count_sqrt);
    let destination_tile = destination.parent_small_tile();

    ruined_waystones
        .iter()
        .filter(|(_, location)| {
            location.dimension == dimensions::OVERWORLD
                && RegionCoordinates::from_ctx(ctx, *location).to_region_index(region.region_count_sqrt) == destination_region
        })
        .min_by_key(|(entity_id, location)| (destination_tile.distance_to(*location), *entity_id))
        .map(|(_, location)| *location)
}

fn watchtower_ids_in_region(ctx: &ReducerContext, region: &WorldRegionState) -> Vec<u64> {
    ctx.db
        .building_state()
        .building_description_id()
        .filter(WATCHTOWER_BUILDING_DESC_ID)
        .filter(|building| {
            ctx.db
                .location_state()
                .entity_id()
                .find(building.entity_id)
                .is_some_and(|location| {
                    RegionCoordinates::from_ctx(ctx, location.coordinates()).to_region_index(region.region_count_sqrt)
                        == region.region_index
                })
        })
        .map(|building| building.entity_id)
        .collect()
}

fn player_overworld_location(ctx: &ReducerContext, player_id: u64) -> Option<SmallHexTile> {
    let mobile = ctx.db.mobile_entity_state().entity_id().find(player_id)?;
    if mobile.dimension == dimensions::OVERWORLD {
        return Some(mobile.coordinates_float().parent_small_tile());
    }
    let dimension = ctx.db.dimension_description_state().dimension_id().find(mobile.dimension)?;
    if let Some(housing) = ctx
        .db
        .player_housing_state()
        .network_entity_id()
        .find(dimension.dimension_network_entity_id)
    {
        return ctx
            .db
            .location_state()
            .entity_id()
            .find(housing.entrance_building_entity_id)
            .map(|location| location.coordinates());
    }
    let network = ctx
        .db
        .dimension_network_state()
        .entity_id()
        .find(dimension.dimension_network_entity_id)?;
    ctx.db
        .location_state()
        .entity_id()
        .find(network.building_id)
        .map(|location| location.coordinates())
}

fn closest_open_neighbor_destination(
    ctx: &ReducerContext,
    region: &WorldRegionState,
    location: SmallHexTile,
) -> Result<FloatHexTile, String> {
    let region_coord = RegionCoordinates::from_region_index(region.region_index, region.region_count_sqrt);
    let min_x = region_coord.x as i32 * region.region_width_chunks as i32 * TerrainChunkState::WIDTH as i32;
    let min_z = region_coord.z as i32 * region.region_height_chunks as i32 * TerrainChunkState::HEIGHT as i32;
    let max_x = min_x + region.region_width_chunks as i32 * TerrainChunkState::WIDTH as i32;
    let max_z = min_z + region.region_height_chunks as i32 * TerrainChunkState::HEIGHT as i32;
    let position = HexCoordinates::from(location.parent_large_tile()).to_offset_coordinates();
    let candidates = [
        (-1, 0, position.x - min_x),
        (1, 0, max_x - position.x),
        (0, -1, position.z - min_z),
        (0, 1, max_z - position.z),
    ];
    let mut best: Option<(i32, FloatHexTile)> = None;

    for (x_offset, z_offset, distance) in candidates {
        let x = region_coord.x as i32 + x_offset;
        let z = region_coord.z as i32 + z_offset;
        if x < 0 || z < 0 || x >= region.region_count_sqrt as i32 || z >= region.region_count_sqrt as i32 {
            continue;
        }

        let neighbor_region_index = RegionCoordinates { x: x as u8, z: z as u8 }.to_region_index(region.region_count_sqrt);
        let Some(control) = ctx.db.region_control_info().region_id().find(neighbor_region_index) else {
            continue;
        };
        if !control.initialized || !control.allow_players {
            continue;
        }

        let destination = OffsetCoordinates {
            x: if x_offset < 0 {
                min_x - TELEPORT_OFFSET_FROM_REGION_EDGE
            } else if x_offset > 0 {
                max_x + TELEPORT_OFFSET_FROM_REGION_EDGE
            } else {
                position.x
            },
            z: if z_offset < 0 {
                min_z - TELEPORT_OFFSET_FROM_REGION_EDGE
            } else if z_offset > 0 {
                max_z + TELEPORT_OFFSET_FROM_REGION_EDGE
            } else {
                position.z
            },
            dimension: dimensions::OVERWORLD,
        };
        let destination = FloatHexTile::from(LargeHexTile::from(HexCoordinates::from(destination)).center_small_tile());
        if RegionCoordinates::from_ctx(ctx, destination).to_region_index(region.region_count_sqrt) != neighbor_region_index {
            continue;
        }
        if best.as_ref().is_none_or(|(best_distance, _)| distance < *best_distance) {
            best = Some((distance, destination));
        }
    }

    best.map(|(_, destination)| destination)
        .ok_or_else(|| "No adjacent region accepts players".into())
}
