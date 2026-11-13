use spacetimedb::{log, ReducerContext};

use crate::{
    game::{coordinates::SmallHexTile, dimensions, handlers::authentication::has_role},
    messages::{
        authentication::Role,
        components::{building_state, location_state, portal_state},
        util::OffsetCoordinatesSmallMessage,
    },
};

#[spacetimedb::reducer]
pub fn admin_dungeon_update_portals(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let dungeon_building_id = 208697589; // Sentinel Dungeon Entrance
    let room_entrance_building_id = 1396283472;
    let room_exit_building_id = 1991870633;

    // find all buildings of type 'dungeon' in the overworld
    for b in ctx.db.building_state().building_description_id().filter(dungeon_building_id) {
        let mut destinations: Vec<SmallHexTile> = Vec::new();

        let dungeon_portal = ctx.db.portal_state().entity_id().find(b.entity_id).unwrap();

        destinations.push(
            OffsetCoordinatesSmallMessage {
                x: dungeon_portal.destination_x,
                z: dungeon_portal.destination_z,
                dimension: dungeon_portal.destination_dimension,
            }
            .into(),
        );

        log::info!("dungeon building = {}", b.entity_id);

        while !destinations.is_empty() {
            let entrance_location = destinations.remove(0);
            let mut closest_dist = i32::MAX;
            let mut closest_portal = None;
            let dimension = entrance_location.dimension;

            for loc in ctx.db.location_state().dimension_filter(dimension) {
                if let Some(exit_portal) = ctx.db.portal_state().entity_id().find(loc.entity_id) {
                    let dist = loc.coordinates().distance_to(entrance_location);
                    if dist < closest_dist {
                        closest_dist = dist;
                        closest_portal = Some(exit_portal.clone());
                    }
                }
            }
            let portal_entity_id = closest_portal.unwrap().entity_id;

            log::info!("processing portal {portal_entity_id}");

            // Change the building type to a 'room entrance portal building'
            let mut entrance_portal_building = ctx.db.building_state().entity_id().find(portal_entity_id).unwrap();

            entrance_portal_building.building_description_id = room_entrance_building_id;

            // Find other portals in the same dimension
            let dimension = ctx.db.location_state().entity_id().find(portal_entity_id).unwrap().dimension;
            log::info!("Updated entrance portal {portal_entity_id}");
            ctx.db.building_state().entity_id().update(entrance_portal_building);

            for loc in ctx.db.location_state().dimension_filter(dimension) {
                if let Some(exit_portal) = ctx.db.portal_state().entity_id().find(loc.entity_id) {
                    if exit_portal.entity_id != portal_entity_id {
                        if exit_portal.destination_dimension != dimensions::OVERWORLD {
                            // Store the destination portal to process
                            destinations.push(
                                OffsetCoordinatesSmallMessage {
                                    x: exit_portal.destination_x,
                                    z: exit_portal.destination_z,
                                    dimension: exit_portal.destination_dimension,
                                }
                                .into(),
                            );
                        }
                        // Update the exit portal to a 'room exit portal building'
                        let mut exit_portal_building = ctx.db.building_state().entity_id().find(exit_portal.entity_id).unwrap();
                        exit_portal_building.building_description_id = room_exit_building_id;
                        log::info!("Updated exit portal {}", exit_portal.entity_id);
                        ctx.db.building_state().entity_id().update(exit_portal_building);
                    }
                }
            }
        }
    }

    Ok(())
}
