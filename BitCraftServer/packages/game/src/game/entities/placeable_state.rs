use std::collections::HashSet;

use spacetimedb::rand::Rng;
use spacetimedb::{ReducerContext, Table};

use crate::messages::components::{LocationState, PlaceableState};
use crate::messages::static_data::*;
use crate::{
    game::{
        autogen::_delete_entity::delete_entity,
        coordinates::*,
        dimensions,
        entities::growth_timer::{delete_placeable_growth_timer, insert_placeable_growth_timer},
        game_state,
        game_state::game_state_filters,
        terrain_chunk::TerrainChunkCache,
    },
    health_state, location_state, placeable_growth_desc, placeable_state, unwrap_or_err, HealthState,
};

impl PlaceableState {
    pub fn get_at_location(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<PlaceableState> {
        LocationState::select_all(ctx, coordinates)
            .filter_map(|ls| ctx.db.placeable_state().entity_id().find(&ls.entity_id))
            .next()
    }

    pub fn distance_to(&self, ctx: &ReducerContext, coordinates: SmallHexTile) -> i32 {
        if let Some(location) = ctx.db.location_state().entity_id().find(&self.entity_id) {
            location.coordinates().distance_to(coordinates)
        } else {
            i32::MAX
        }
    }

    pub fn spawn(
        ctx: &ReducerContext,
        placeable_id: i32,
        owner_entity_id: u64,
        coordinates: SmallHexTile,
        direction_index: i32,
    ) -> Result<(), String> {
        let placeable_desc = unwrap_or_err!(ctx.db.placeable_desc().id().find(&placeable_id), "Unknown placeable");
        let entity_id = game_state::create_entity(ctx);

        ctx.db.placeable_state().try_insert(PlaceableState {
            entity_id,
            owner_entity_id,
            placeable_id,
            direction_index,
        })?;

        ctx.db.health_state().try_insert(HealthState {
            entity_id,
            last_health_decrease_timestamp: ctx.timestamp,
            health: placeable_desc.max_health as f32,
            died_timestamp: 0,
        })?;

        game_state::insert_location(ctx, entity_id, coordinates.to_offset_coordinates());
        Self::add_growth_timer(ctx, entity_id, placeable_id);

        Ok(())
    }

    pub fn despawn(&self, ctx: &ReducerContext) {
        delete_placeable_growth_timer(ctx, self.entity_id);
        delete_entity(ctx, self.entity_id);
    }

    pub fn spawn_in_radius_band(
        ctx: &ReducerContext,
        placeable_id: i32,
        owner_entity_id: u64,
        center: SmallHexTile,
        direction_index: i32,
        min_radius: i32,
        max_radius: i32,
    ) -> Result<bool, String> {
        if placeable_id == 0 {
            return Ok(false);
        }

        let placeable_desc = unwrap_or_err!(ctx.db.placeable_desc().id().find(&placeable_id), "Unknown placeable");
        let mut terrain_cache = TerrainChunkCache::empty();
        let min_radius = min_radius.max(0);
        let max_radius = max_radius.max(min_radius);

        if min_radius == 0 && max_radius == 0 {
            if Self::is_valid_spawn_tile(ctx, &mut terrain_cache, &placeable_desc, owner_entity_id, center) {
                Self::spawn(ctx, placeable_id, owner_entity_id, center, direction_index)?;
                return Ok(true);
            }

            return Ok(false);
        }

        let mut sampled = HashSet::new();
        for _ in 0..SmallHexTile::tile_count_between_radius(min_radius, max_radius) {
            let coordinates = SmallHexTile::random_tile_between_radius(ctx, center, min_radius, max_radius, &mut sampled);

            if !Self::is_valid_spawn_tile(ctx, &mut terrain_cache, &placeable_desc, owner_entity_id, coordinates) {
                continue;
            }

            Self::spawn(ctx, placeable_id, owner_entity_id, coordinates, direction_index)?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn spawn_growth_outcome_with_fallback(
        ctx: &ReducerContext,
        owner_entity_id: u64,
        center: SmallHexTile,
        direction_index: i32,
        outcome: &PlaceableGrowthOutcomeV2,
    ) -> Result<(), String> {
        if outcome.placeable_id == 0 {
            return Ok(());
        }

        if Self::spawn_in_radius_band(
            ctx,
            outcome.placeable_id,
            owner_entity_id,
            center,
            direction_index,
            outcome.radius_min,
            outcome.radius_max,
        )? {
            return Ok(());
        }

        let placeable_desc = unwrap_or_err!(ctx.db.placeable_desc().id().find(&outcome.placeable_id), "Unknown placeable");
        let mut terrain_cache = TerrainChunkCache::empty();
        if !Self::is_valid_spawn_tile(ctx, &mut terrain_cache, &placeable_desc, owner_entity_id, center) {
            return Ok(());
        }

        Self::spawn(ctx, outcome.placeable_id, owner_entity_id, center, direction_index)
    }

    pub fn pick_growth_outcome(ctx: &ReducerContext, outcomes: Option<Vec<PlaceableGrowthOutcomeV2>>) -> Option<PlaceableGrowthOutcomeV2> {
        let outcomes = if let Some(outcomes) = outcomes {
            outcomes
        } else {
            return None;
        };

        if outcomes.is_empty() {
            return None;
        }

        let mut sum = 0.0;
        for outcome in &outcomes {
            sum += outcome.probability;
        }

        if sum <= 0.0 {
            return None;
        }

        let mut rnd = ctx.rng().gen_range(0.0..=sum);
        for outcome in outcomes {
            rnd -= outcome.probability;
            if rnd <= 0.0 {
                return Some(outcome.clone());
            }
        }

        None
    }

    fn add_growth_timer(ctx: &ReducerContext, entity_id: u64, placeable_id: i32) {
        if let Some(growth) = ctx.db.placeable_growth_desc().placeable_id().find(&placeable_id) {
            let _ = insert_placeable_growth_timer(ctx, entity_id, &growth);
        }
    }

    pub fn validate_spawn_terrain(
        terrain_cache: &mut TerrainChunkCache,
        ctx: &ReducerContext,
        coordinates: SmallHexTile,
        placeable_desc: &PlaceableDesc,
    ) -> Result<(), String> {
        let terrain = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
            "Invalid location"
        );

        if terrain.is_submerged() {
            if !placeable_desc.spawns_in_water {
                return Err("This placeable must be placed on land".into());
            }

            let water_depth = terrain.water_depth() as i32;
            if water_depth < placeable_desc.water_depth_min {
                return Err("The water level is too shallow.".into());
            }
            if water_depth > placeable_desc.water_depth_max {
                return Err("The water level is too deep.".into());
            }
        } else {
            if !placeable_desc.spawns_on_land {
                return Err("This placeable must be placed in water".into());
            }

            let elevation = terrain.elevation as i32;
            if elevation < placeable_desc.land_elevation_min {
                return Err("This placeable can't be placed that low".into());
            }
            if elevation > placeable_desc.land_elevation_max {
                return Err("This placeable can't be placed that high".into());
            }
        }

        Ok(())
    }

    fn is_valid_spawn_tile(
        ctx: &ReducerContext,
        terrain_cache: &mut TerrainChunkCache,
        placeable_desc: &PlaceableDesc,
        owner_entity_id: u64,
        coordinates: SmallHexTile,
    ) -> bool {
        if Self::validate_spawn_terrain(terrain_cache, ctx, coordinates, placeable_desc).is_err() {
            return false;
        }

        if game_state_filters::has_blocking_footprint(ctx, coordinates) {
            return false;
        }

        if LocationState::select_all(ctx, &coordinates)
            .filter_map(|location| ctx.db.placeable_state().entity_id().find(&location.entity_id))
            .any(|placeable| placeable.owner_entity_id == owner_entity_id)
        {
            return false;
        }

        if coordinates.dimension != dimensions::OVERWORLD && !game_state_filters::is_interior_tile_walkable(ctx, coordinates) {
            return false;
        }

        true
    }
}
