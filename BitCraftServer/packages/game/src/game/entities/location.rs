//use spacetimedb::query;

use spacetimedb::{ReducerContext, Timestamp};

use crate::game::coordinates::FloatHexTile;
use crate::game::coordinates::OffsetCoordinatesFloat;
use crate::game::coordinates::*;
use crate::game::dimensions;
use crate::game::game_state;
use crate::location_state;
use crate::messages::components::player_action_state;
use crate::messages::components::PlayerActionType;
use crate::messages::components::TerrainChunkState;
pub use crate::messages::components::{LocationState, MobileEntityState};
use crate::mobile_entity_state;

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

    pub fn set_location(&mut self, location: OffsetCoordinatesSmall) {
        let offset_coordinates = SmallHexTile::from(location);
        self.x = location.x;
        self.z = location.z;
        self.dimension = location.dimension;
        self.chunk_index = TerrainChunkState::chunk_index_from_coords(&offset_coordinates.chunk_coordinates());
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

    pub fn select_all<'a>(ctx: &'a ReducerContext, coordinates: SmallHexTile) -> impl Iterator<Item = MobileEntityState> + use<'a> {
        ctx.db
            .mobile_entity_state()
            .chunk_index()
            .filter(coordinates.chunk_coordinates().chunk_index())
            .filter(move |c| {
                let s = c.coordinates();
                s.x == coordinates.x && s.z == coordinates.z
            })
    }

    pub fn select_all_in_interior_dimension_iter(ctx: &ReducerContext, dimension: u32) -> impl Iterator<Item = Self> {
        if dimension == dimensions::OVERWORLD {
            panic!("This function should only be used for interiors");
        }

        let chunk_coordinates = ChunkCoordinates { x: 0, z: 0, dimension };
        Self::select_all_in_chunk_iter(ctx, &chunk_coordinates)
    }

    pub fn select_all_in_chunk_iter(ctx: &ReducerContext, chunk_coordinates: &ChunkCoordinates) -> impl Iterator<Item = Self> {
        let chunk_index = TerrainChunkState::chunk_index_from_coords(chunk_coordinates);
        ctx.db.mobile_entity_state().chunk_index().filter(chunk_index)
    }

    pub fn interpolated_location(&self, ctx: &ReducerContext) -> FloatHexTile {
        //This may not be 100% accurate / validated - we get duration from clients and only do basic validation on it
        let origin = self.coordinates_float();
        let distination = self.destination_float();
        if origin.x == distination.x && origin.z == distination.z {
            origin
        } else {
            let action_state = match ctx
                .db
                .player_action_state()
                .entity_id()
                .filter(self.entity_id).find(|a| a.action_type == PlayerActionType::PlayerMove || a.action_type == PlayerActionType::DeployableMove)
            {
                Some(a) => a,
                None => return origin,
            };
            let travel_time = action_state.duration as f32 / 1000.0;
            let time_diff = ((game_state::unix_ms(ctx.timestamp) - self.timestamp) as f32 / 1000.0).clamp(0.0, travel_time);
            let t = time_diff / travel_time;
            FloatHexTile::lerp(&origin, &distination, t)
        }
    }

    //pub fn cur_distance_traveled(&self, speed: f32) -> f32 {
    //    let origin = self.coordinates_float();
    //    let distination = self.destination_float();
    //    if origin.x == distination.x && origin.z == distination.z {
    //        return 0.0;
    //    } else {
    //        let travel_time = move_validation_helpers::travel_time(&origin, &distination, speed);
    //        let travel_time = travel_time;
    //        let time_diff = ((game_state::unix_ms() - self.timestamp) as f32 / 1000.0).clamp(0.0, travel_time);
    //        return speed * time_diff / travel_time;
    //    };
    //}
    //
    //pub fn cur_coord_and_distance_traveled(&self, speed: f32) -> (FloatHexTile, f32) {
    //    let origin = self.coordinates_float();
    //    let distination: crate::messages::util::FloatHexTileMessage = self.destination_float();
    //    if origin.x == distination.x && origin.z == distination.z {
    //        return (origin.clone(), 0.0);
    //    } else {
    //        let travel_time = move_validation_helpers::travel_time(&origin, &distination, speed);
    //        let time_diff = ((game_state::unix_ms() - self.timestamp) as f32 / 1000.0).clamp(0.0, travel_time);
    //        let t = time_diff / travel_time;
    //        return (FloatHexTile::lerp(&origin, &distination, t), speed * t);
    //    };
    //}
}

impl crate::messages::components::location_state__TableHandle {
    pub fn dimension_delete(&self, dimension: u32) -> u64 {
        if dimension == dimensions::OVERWORLD {
            panic!("This function should only be used for interiors");
        }

        let chunk_coordinates = ChunkCoordinates { x: 0, z: 0, dimension };
        self.chunk_index().delete(chunk_coordinates.chunk_index())
    }

    pub fn dimension_filter(&self, dimension: u32) -> impl Iterator<Item = LocationState> {
        if dimension == dimensions::OVERWORLD {
            panic!("This function should only be used for interiors");
        }

        let chunk_coordinates = ChunkCoordinates { x: 0, z: 0, dimension };
        self.chunk_index().filter(chunk_coordinates.chunk_index())
    }
}
