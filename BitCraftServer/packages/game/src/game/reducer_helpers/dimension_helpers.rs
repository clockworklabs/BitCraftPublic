use crate::{
    dimension_description_state,
    game::{
        coordinates::{OffsetCoordinatesFloat, OffsetCoordinatesLarge, OffsetCoordinatesSmall},
        dimensions,
    },
    messages::generic::world_region_state,
};
use spacetimedb::ReducerContext;

use crate::messages::components::TerrainChunkState;

pub fn get_dimension_size(ctx: &ReducerContext, dimension: &u32) -> (i32, i32) {
    if *dimension == dimensions::OVERWORLD {
        let region = ctx.db.world_region_state().id().find(0).unwrap();

        return (region.region_width_chunks as i32, region.region_height_chunks as i32);
    }

    let dimension_desc = ctx.db.dimension_description_state().dimension_id().find(dimension).unwrap();

    (
        dimension_desc.dimension_size_large_x as i32,
        dimension_desc.dimension_size_large_z as i32,
    )
}

pub fn get_dimension_bounds(ctx: &ReducerContext, dimension: &u32) -> (OffsetCoordinatesLarge, OffsetCoordinatesLarge) {
    if *dimension == dimensions::OVERWORLD {
        let region = ctx.db.world_region_state().id().find(0).unwrap();

        let min: OffsetCoordinatesLarge = OffsetCoordinatesLarge {
            x: region.region_min_chunk_x as i32 * TerrainChunkState::WIDTH as i32,
            z: region.region_min_chunk_z as i32 * TerrainChunkState::HEIGHT as i32,
            dimension: *dimension,
        };
        let max: OffsetCoordinatesLarge = OffsetCoordinatesLarge {
            x: (region.region_min_chunk_x as i32 + region.region_width_chunks as i32) * TerrainChunkState::WIDTH as i32 - 1,
            z: (region.region_min_chunk_z as i32 + region.region_height_chunks as i32) * TerrainChunkState::WIDTH as i32 - 1,
            dimension: *dimension,
        };
        return (min, max);
    }

    let dimension_desc = ctx.db.dimension_description_state().dimension_id().find(dimension).unwrap();

    let min: OffsetCoordinatesLarge = OffsetCoordinatesLarge {
        x: 0,
        z: 0,
        dimension: *dimension,
    };
    let max: OffsetCoordinatesLarge = OffsetCoordinatesLarge {
        x: (dimension_desc.dimension_size_large_x as i32) * TerrainChunkState::WIDTH as i32 - 1,
        z: (dimension_desc.dimension_size_large_z as i32) * TerrainChunkState::HEIGHT as i32 - 1,
        dimension: *dimension,
    };

    (min, max)
}

pub fn is_within_dimension_bounds(
    offset_coordinates_float: &OffsetCoordinatesFloat,
    bounds: (OffsetCoordinatesLarge, OffsetCoordinatesLarge),
) -> bool {
    let min = OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(bounds.0));
    let max = OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(bounds.1));

    offset_coordinates_float.x >= min.x
        && offset_coordinates_float.x <= max.x
        && offset_coordinates_float.z >= min.z
        && offset_coordinates_float.z <= max.z
}

pub fn is_within_dimension_bounds_small(
    offset_coordinates_small: &OffsetCoordinatesSmall,
    bounds: (OffsetCoordinatesLarge, OffsetCoordinatesLarge),
) -> bool {
    let min = OffsetCoordinatesSmall::from(bounds.0);
    let max = OffsetCoordinatesSmall::from(bounds.1);

    offset_coordinates_small.x >= min.x
        && offset_coordinates_small.x <= max.x
        && offset_coordinates_small.z >= min.z
        && offset_coordinates_small.z <= max.z
}

pub fn clamp_within_dimension_bounds(
    offset_coordinates_float: &mut OffsetCoordinatesFloat,
    bounds: (OffsetCoordinatesLarge, OffsetCoordinatesLarge),
) {
    let min = OffsetCoordinatesLarge {
        x: bounds.0.x + 1,
        z: bounds.0.z + 1,
        dimension: bounds.0.dimension,
    };
    let max = OffsetCoordinatesLarge {
        x: bounds.1.x - 1,
        z: bounds.1.z - 1,
        dimension: bounds.1.dimension,
    };

    let min_float = OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(min));
    let max_float = OffsetCoordinatesFloat::from(OffsetCoordinatesSmall::from(max));

    offset_coordinates_float.x = offset_coordinates_float.x.clamp(min_float.x, max_float.x);
    offset_coordinates_float.z = offset_coordinates_float.z.clamp(min_float.z, max_float.z);
}
