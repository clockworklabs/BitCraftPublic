use crate::{
    building_portal_desc, building_state,
    game::{coordinates::SmallHexTile, game_state::game_state_filters, unity_helpers::vector2::Vector2},
    BuildingPortalDesc,
};
use spacetimedb::ReducerContext;

pub fn distance_to_segment(point: Vector2, seg1: Vector2, seg2: Vector2) -> f32 {
    let l2 = (seg1 - seg2).sqr_magnitude();
    if l2 < 0.1 {
        return (point - seg1).magnitude();
    }

    let t: f32 = (Vector2::dot(&(point - seg1), &(seg2 - seg1)) / l2).clamp(0.0, 1.0);
    let projection: Vector2 = seg1 + (seg2 - seg1) * t;
    (point - projection).magnitude()
}

pub fn distance_to_portal(ctx: &ReducerContext, building_id: u64, coord: SmallHexTile, mounting_deployable: bool) -> Result<i32, String> {
    let building = ctx.db.building_state().entity_id().find(building_id).unwrap();
    let building_coord = game_state_filters::coordinates(ctx, building_id);
    let building_portals: Vec<BuildingPortalDesc> = ctx
        .db
        .building_portal_desc()
        .building_id()
        .filter(building.building_description_id)
        .collect();

    let mut min_distance = i32::MAX;
    for portal in building_portals {
        if mounting_deployable && !portal.allow_deployables {
            continue;
        }
        let oc_coord = building_coord
            + SmallHexTile {
                x: portal.pos_x,
                z: portal.pos_z,
                dimension: 0,
            };
        let coord_rotated = oc_coord.rotate_around(&building_coord, building.direction_index / 2);
        min_distance = min_distance.min(coord.distance_to(coord_rotated));
    }
    if min_distance == i32::MAX {
        return Err("This portal does not allow deployables".into());
    }
    Ok(min_distance)
}
