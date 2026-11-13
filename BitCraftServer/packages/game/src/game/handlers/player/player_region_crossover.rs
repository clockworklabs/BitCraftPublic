use bitcraft_macro::feature_gate;
use crate::game::coordinates::hex_coordinates::HexCoordinates;
use crate::game::coordinates::offset_coordinates::OffsetCoordinates;
use crate::game::coordinates::region_coordinates::RegionCoordinates;
use crate::game::dimensions;
use crate::game::unity_helpers::vector2int::Vector2Int;
use crate::messages::generic::world_region_state;
use crate::params;
use crate::{
    game::{coordinates::*, game_state},
    messages::{components::*, static_data::*},
};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_region_crossover(ctx: &ReducerContext) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let region = ctx.db.world_region_state().iter().next().unwrap();
    let region_coord = RegionCoordinates::from_region_index(region.region_index, region.region_count_sqrt);
    let vec2 = Vector2Int::new(region_coord.x as i32, region_coord.z as i32);
    let chunk_size = Vector2Int::new(TerrainChunkState::WIDTH as i32, TerrainChunkState::HEIGHT as i32);
    let region_size = Vector2Int::new(region.region_width_chunks as i32, region.region_height_chunks as i32);
    let min_coord = vec2 * region_size * chunk_size;
    let max_coord = min_coord + region_size * chunk_size;

    let mobile_entity = ctx.db.mobile_entity_state().entity_id().find(actor_id).unwrap();
    if mobile_entity.dimension != dimensions::OVERWORLD {
        return Err("Transfering regions is only possible in Overworld".into());
    }
    let pos = HexCoordinates::from(mobile_entity.coordinates_float().parent_large_tile()).to_offset_coordinates();
    let min_distance = params!(ctx).region_crossover_distance_large_tiles;

    let distance_left = pos.x - min_coord.x;
    let distance_down = pos.z - min_coord.y;
    let distance_right = max_coord.x - pos.x;
    let distance_up = max_coord.y - pos.z;

    let z_off: i32 = if distance_up <= min_distance && region_coord.z < region.region_count_sqrt - 1 {
        1 //North
    } else if distance_down <= min_distance && region_coord.z > 0 {
        -1 //South
    } else {
        0
    };
    let x_off: i32 = if distance_right <= min_distance && region_coord.x < region.region_count_sqrt - 1 {
        1 //East
    } else if distance_left <= min_distance && region_coord.x > 0 {
        -1 //West
    } else {
        0
    };

    if x_off == 0 && z_off == 0 {
        return Err("Not near region edge".into());
    }

    let teleport_offset_from_edge = 2;
    let target_coord = OffsetCoordinates {
        x: if x_off < 0 {
            min_coord.x - teleport_offset_from_edge
        } else if x_off > 0 {
            max_coord.x + teleport_offset_from_edge
        } else {
            pos.x
        },
        z: if z_off < 0 {
            min_coord.y - teleport_offset_from_edge
        } else if z_off > 0 {
            max_coord.y + teleport_offset_from_edge
        } else {
            pos.z
        },
        dimension: dimensions::OVERWORLD,
    };

    let teleport_location = FloatHexTile::from(LargeHexTile::from(HexCoordinates::from(target_coord)).center_small_tile());
    crate::inter_module::transfer_player::send_message(ctx, actor_id, teleport_location, true, 0.0)?;

    Ok(())
}
