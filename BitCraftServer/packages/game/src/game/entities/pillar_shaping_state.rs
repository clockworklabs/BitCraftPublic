use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::location_state;
use crate::messages::components::LocationState;
use crate::messages::components::{pillar_shaping_state, PillarShapingState};
use spacetimedb::{ReducerContext, Table};

impl PillarShapingState {
    pub fn get_at_location(ctx: &ReducerContext, coordinates: &LargeHexTile) -> Option<PillarShapingState> {
        LocationState::select_all(ctx, &coordinates.center_small_tile())
            .filter_map(|ls| ctx.db.pillar_shaping_state().entity_id().find(ls.entity_id))
            .next()
    }

    pub fn delete_pillar_shaping(ctx: &ReducerContext, entity_id: u64) {
        ctx.db.location_state().entity_id().delete(entity_id);
        ctx.db.pillar_shaping_state().entity_id().delete(entity_id);
    }

    pub fn create_pillar_shape_unsafe(ctx: &ReducerContext, coordinates: LargeHexTile, pillar_type_id: i32) -> Result<(), String> {
        let mut terrain_cache = TerrainChunkCache::empty();

        // Verify distance to paving target
        let target_coord = coordinates.center_small_tile();

        if terrain_cache.get_terrain_cell(ctx, &coordinates).is_none() {
            return Err("Invalid coordinates".into());
        }

        // Delete existing pillar
        if let Some(existing_pillar_shaping) = Self::get_at_location(ctx, &coordinates) {
            if existing_pillar_shaping.pillar_type_id == pillar_type_id {
                return Err("This pillar already has this decoration".into());
            }
            Self::delete_pillar_shaping(ctx, existing_pillar_shaping.entity_id);
        }

        // Create pillar shaping
        let entity_id = game_state::create_entity(ctx);

        // location
        let offset = target_coord.to_offset_coordinates();
        game_state::insert_location(ctx, entity_id, offset);

        // pillar shaping entity
        let pillar_shaping = PillarShapingState { entity_id, pillar_type_id };

        if ctx.db.pillar_shaping_state().try_insert(pillar_shaping).is_err() {
            return Err("Failed to insert pillar_ haping".into());
        }

        Ok(())
    }
}
