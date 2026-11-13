use bitcraft_macro::feature_gate;
use crate::{
    building_desc, building_function_type_mapping_desc, building_state,
    game::{
        dimensions,
        game_state::{self, game_state_filters},
    },
    global_search_state, location_state, unwrap_or_err, DimensionNetworkState, GlobalSearchState, LocationState, SmallHexTile,
};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn search_for_closest_building(ctx: &ReducerContext, building_description_ids: Vec<i32>) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let actor_coord = get_player_location(ctx, actor_id);

    let mut shortest_distance = i32::MAX;
    let mut closest_id = 0;
    let mut closest = None;
    for building_description_id in building_description_ids {
        for building_state in ctx.db.building_state().building_description_id().filter(building_description_id) {
            let building_entity_id = building_state.entity_id;
            let building_ls = unwrap_or_err!(
                ctx.db.location_state().entity_id().find(building_entity_id),
                "Couldn't locate building."
            );
            let building_coord = building_ls.coordinates();

            if building_coord.dimension != dimensions::OVERWORLD {
                continue;
            }

            let dist = building_coord.distance_to(actor_coord);
            if dist < shortest_distance {
                closest_id = building_description_id;
                shortest_distance = dist;
                closest = Some(building_ls);
            }
        }
    }

    if closest.is_none() {
        return Ok(());
    }

    let building_desc = unwrap_or_err!(ctx.db.building_desc().id().find(closest_id), "Invalid Building Description.");

    let ls: LocationState = closest.unwrap();
    let coords = ls.coordinates();
    let result = GlobalSearchState {
        entity_id: actor_id,
        found_entity_id: ls.entity_id,
        found_entity_name: building_desc.name,
        x: coords.x,
        z: coords.z,
        timestamp: ctx.timestamp,
    };

    if ctx.db.global_search_state().entity_id().find(actor_id).is_some() {
        ctx.db.global_search_state().entity_id().update(result);
    } else {
        // player must have changed region during tutorial
        ctx.db.global_search_state().try_insert(result)?;
    }
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn search_for_closest_building_type(ctx: &ReducerContext, building_type_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    let actor_coord = get_player_location(ctx, actor_id);

    let mut shortest_distance = i32::MAX;
    let mut closest = None;

    let mapping = unwrap_or_err!(
        ctx.db.building_function_type_mapping_desc().type_id().find(building_type_id),
        "Building type doesn't exist."
    );

    for building_desc_id in mapping.desc_ids {
        for building_state in ctx.db.building_state().building_description_id().filter(building_desc_id) {
            let building_entity_id = building_state.entity_id;
            let building_ls = unwrap_or_err!(
                ctx.db.location_state().entity_id().find(building_entity_id),
                "Couldn't locate building."
            );
            let building_coord = building_ls.coordinates();

            if building_coord.dimension != dimensions::OVERWORLD {
                continue;
            }

            let dist = building_coord.distance_to(actor_coord);
            if dist < shortest_distance {
                shortest_distance = dist;
                closest = Some(building_ls);
            }
        }
    }

    if closest.is_none() {
        return Ok(());
    }

    let ls: LocationState = closest.unwrap();
    let coords = ls.coordinates();
    let result = GlobalSearchState {
        entity_id: actor_id,
        found_entity_id: ls.entity_id,
        found_entity_name: ctx
            .db
            .building_desc()
            .id()
            .find(
                ctx.db
                    .building_state()
                    .entity_id()
                    .find(ls.entity_id)
                    .unwrap()
                    .building_description_id,
            )
            .unwrap()
            .name,
        x: coords.x,
        z: coords.z,
        timestamp: ctx.timestamp,
    };

    if ctx.db.global_search_state().entity_id().find(actor_id).is_some() {
        ctx.db.global_search_state().entity_id().update(result);
    } else {
        // player must have changed region during tutorial
        ctx.db.global_search_state().try_insert(result)?;
    }

    Ok(())
}

fn get_player_location(ctx: &ReducerContext, actor_id: u64) -> SmallHexTile {
    let location = game_state_filters::coordinates_any(ctx, actor_id);

    if location.dimension != dimensions::OVERWORLD {
        if let Some(dimension_network_state) = DimensionNetworkState::get(ctx, location.dimension) {
            if let Some(loc) = ctx.db.location_state().entity_id().find(dimension_network_state.building_id) {
                return loc.coordinates();
            }
        }
    }

    location
}
