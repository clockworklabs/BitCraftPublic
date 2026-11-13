use hex_direction::HexDirection;
use spacetimedb::ReducerContext;

use crate::{game::coordinates::*, messages::components::*};

pub fn coordinates(ctx: &ReducerContext, entity_id: u64) -> SmallHexTile {
    if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
        return location.coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn offset_coordinates(ctx: &ReducerContext, entity_id: u64) -> OffsetCoordinatesSmall {
    if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
        return location.offset_coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_float(ctx: &ReducerContext, entity_id: u64) -> FloatHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        return location.coordinates_float();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn offset_coordinates_float(ctx: &ReducerContext, entity_id: u64) -> OffsetCoordinatesFloat {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        return location.offset_coordinates_float();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_any(ctx: &ReducerContext, entity_id: u64) -> SmallHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        return location.coordinates();
    }
    if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
        return location.coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_any_float(ctx: &ReducerContext, entity_id: u64) -> FloatHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        return location.coordinates_float();
    }
    if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
        return location.coordinates().into();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn chunk_indexes_in_radius(coord: SmallHexTile, radius: i32) -> impl Iterator<Item = u64> {
    let chunk = coord.chunk_coordinates();
    let mut min_x = chunk.x;
    let mut min_z = chunk.z;
    let mut max_x = chunk.x;
    let mut max_z = chunk.z;

    let mut direction = HexDirection::NE;
    for _i in 0..6 {
        let chunk = coord.neighbor_n(direction, radius).chunk_coordinates();
        min_x = min_x.min(chunk.x);
        min_z = min_z.min(chunk.z);
        max_x = max_x.max(chunk.x);
        max_z = max_z.max(chunk.z);
        direction = HexDirection::previous_flat(direction);
    }

    let dimension = coord.dimension;
    (min_x..=max_x).flat_map(move |x| (min_z..=max_z).map(move |z| ChunkCoordinates { x, z, dimension }.chunk_index()))
}

pub fn get_location_for_entity(ctx: &ReducerContext, entity_id: u64) -> Option<SmallHexTile> {
    if let Some(loc) = ctx.db.location_state().entity_id().find(entity_id) {
        return Some(loc.coordinates());
    }
    if let Some(loc) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        return Some(loc.coordinates());
    }
    // Progressive actions locations are their building's
    if let Some(action) = ctx.db.progressive_action_state().entity_id().find(entity_id) {
        if let Some(loc) = ctx.db.location_state().entity_id().find(action.building_entity_id) {
            return Some(loc.coordinates());
        }
    }
    // Inventory locations are their owner's
    if let Some(inventory) = ctx.db.inventory_state().entity_id().find(entity_id) {
        if let Some(loc) = ctx.db.mobile_entity_state().entity_id().find(inventory.owner_entity_id) {
            // Player or Deployable Inventory
            return Some(loc.coordinates());
        }
        if let Some(loc) = ctx.db.location_state().entity_id().find(inventory.owner_entity_id) {
            // Building Inventory
            return Some(loc.coordinates());
        }
    }
    // This is a global entity with no location
    None
}
