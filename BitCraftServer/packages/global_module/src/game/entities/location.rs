//use spacetimedb::query;

use spacetimedb::ReducerContext;
use spacetimedb::Timestamp;

use crate::game::coordinates::FloatHexTile;
use crate::game::coordinates::OffsetCoordinatesFloat;
use crate::game::coordinates::*;
use crate::game::dimensions;
use crate::game::game_state;
use crate::location_state;
use crate::messages::components::TerrainChunkState;
pub use crate::messages::components::{LocationState, MobileEntityState};

impl Copy for LocationState {}

impl LocationState {
    pub fn coordinates(&self) -> SmallHexTile {
        SmallHexTile::from(OffsetCoordinatesSmall {
            x: self.x,
            z: self.z,
            dimension: self.dimension,
        })
    }

    pub fn offset_coordinates(&self) -> OffsetCoordinatesSmall {
        OffsetCoordinatesSmall {
            x: self.x,
            z: self.z,
            dimension: self.dimension,
        }
    }

    pub fn select_all(ctx: &ReducerContext, coordinates: &SmallHexTile) -> impl Iterator<Item = LocationState> {
        let loc_chunk_index = coordinates.chunk_coordinates().chunk_index();
        let offset = OffsetCoordinatesSmall::from(coordinates);
        ctx.db
            .location_state()
            .x_z_chunk_index()
            .filter((offset.x, offset.z, loc_chunk_index))
    }

    pub fn new(entity_id: u64, offset_coordinates: OffsetCoordinatesSmall) -> LocationState {
        let chunk_coordinates = ChunkCoordinates::from(SmallHexTile::from(&offset_coordinates));
        LocationState {
            entity_id,
            chunk_index: TerrainChunkState::chunk_index_from_coords(&chunk_coordinates),
            x: offset_coordinates.x,
            z: offset_coordinates.z,
            dimension: offset_coordinates.dimension,
        }
    }

    pub fn select_all_in_chunk_iter(ctx: &ReducerContext, chunk_coordinates: &ChunkCoordinates) -> impl Iterator<Item = LocationState> {
        let chunk_index = TerrainChunkState::chunk_index_from_coords(chunk_coordinates);
        ctx.db.location_state().chunk_index().filter(chunk_index)
    }

    pub fn select_all_in_interior_dimension_iter(ctx: &ReducerContext, dimension: u32) -> impl Iterator<Item = LocationState> {
        if dimension == dimensions::OVERWORLD {
            panic!("This function should only be used for interiors");
        }

        let chunk_coordinates = ChunkCoordinates { x: 0, z: 0, dimension };
        LocationState::select_all_in_chunk_iter(ctx, &chunk_coordinates)
    }
}

impl MobileEntityState {
    pub fn for_location(entity_id: u64, location: OffsetCoordinatesFloat, now: Timestamp) -> MobileEntityState {
        MobileEntityState {
            entity_id,
            chunk_index: TerrainChunkState::chunk_index_from_coords(
                &SmallHexTile::from(OffsetCoordinatesSmall::from(location)).chunk_coordinates(),
            ),
            location_x: location.x,
            location_z: location.z,
            destination_x: location.x,
            destination_z: location.z,
            dimension: location.dimension,
            timestamp: game_state::unix_ms(now),
            is_walking: false,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }

    pub fn for_location_and_destination(
        entity_id: u64,
        location: OffsetCoordinatesFloat,
        destination: OffsetCoordinatesFloat,
        now: Timestamp,
    ) -> MobileEntityState {
        debug_assert!(location.dimension == destination.dimension);
        MobileEntityState {
            entity_id,
            chunk_index: TerrainChunkState::chunk_index_from_coords(
                &SmallHexTile::from(OffsetCoordinatesSmall::from(location)).chunk_coordinates(),
            ),
            location_x: location.x,
            location_z: location.z,
            destination_x: destination.x,
            destination_z: destination.z,
            dimension: location.dimension,
            timestamp: game_state::unix_ms(now),
            is_walking: false,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }

    pub fn set_location(&mut self, location: OffsetCoordinatesFloat) {
        let offset_coordinates = SmallHexTile::from(OffsetCoordinatesSmall::from(location));
        self.location_x = location.x;
        self.location_z = location.z;
        self.dimension = location.dimension;
        self.chunk_index = TerrainChunkState::chunk_index_from_coords(&offset_coordinates.chunk_coordinates());
    }

    pub fn set_destination(&mut self, location: OffsetCoordinatesFloat) {
        self.destination_x = location.x;
        self.destination_z = location.z;
    }

    pub fn coordinates(&self) -> SmallHexTile {
        let c = OffsetCoordinatesFloat {
            x: self.location_x,
            z: self.location_z,
            dimension: self.dimension,
        };
        SmallHexTile::from(OffsetCoordinatesSmall::from(c))
    }

    pub fn destination(&self) -> SmallHexTile {
        let c = OffsetCoordinatesFloat {
            x: self.destination_x,
            z: self.destination_z,
            dimension: self.dimension,
        };
        SmallHexTile::from(OffsetCoordinatesSmall::from(c))
    }

    pub fn offset_coordinates(&self) -> OffsetCoordinatesSmall {
        let c = OffsetCoordinatesFloat {
            x: self.location_x,
            z: self.location_z,
            dimension: self.dimension,
        };
        c.into()
    }

    pub fn coordinates_float(&self) -> FloatHexTile {
        FloatHexTile::from(self.offset_coordinates_float())
    }

    pub fn offset_coordinates_float(&self) -> OffsetCoordinatesFloat {
        OffsetCoordinatesFloat {
            x: self.location_x,
            z: self.location_z,
            dimension: self.dimension,
        }
    }

    pub fn destination_float(&self) -> FloatHexTile {
        FloatHexTile::from(self.offset_destination_float())
    }

    pub fn offset_destination_float(&self) -> OffsetCoordinatesFloat {
        OffsetCoordinatesFloat {
            x: self.destination_x,
            z: self.destination_z,
            dimension: self.dimension,
        }
    }

    pub fn to_location_state(&self) -> LocationState {
        LocationState::new(self.entity_id, self.offset_coordinates())
    }
}
