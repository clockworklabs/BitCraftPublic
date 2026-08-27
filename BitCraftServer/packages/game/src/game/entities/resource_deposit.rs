use std::collections::HashSet;

use crate::agents::{crumb_trail_clean_up_agent, resources_regen};
use crate::game::autogen::_delete_entity::delete_entity;
use crate::game::entities::growth_timer::{delete_resource_growth_timer, insert_resource_growth_timer, resource_growth_timer};
use crate::game::handlers::resource::respawn_resource_in_chunk::{respawn_resource_in_chunk_timer, RespawnResourceInChunkTimer};
use crate::game::reducer_helpers::timer_helpers::now_plus_secs_f32;
use crate::game::reducer_helpers::{footprint_helpers, footprint_helpers::delete_footprint};
use crate::game::{coordinates::*, dimensions, game_state, terrain_chunk::TerrainChunkCache};
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::{
    distant_visible_entity, light_source_state, location_state, DistantVisibleEntity, FootprintTileState, LightSourceState, LocationState,
    ResourceState,
};
use crate::messages::generic::{resource_count, ResourceCount};
use crate::messages::static_data::*;
use crate::messages::util::SmallHexTileMessage;
use crate::{building_state, growth_state, resource_health_state, resource_state, unwrap_or_err, ResourceHealthState};
use crate::{AttachedHerdsState, HerdState};
use spacetimedb::rand::Rng;
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::table(name = resource_spawn_timer, scheduled(resource_spawn_scheduled, at = scheduled_at))]
pub struct ResourceSpawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: Option<u64>,
    pub resource_id: i32,
    pub coordinates: SmallHexTile,
    pub direction_index: i32,
    pub health: i32,
    pub check_buildings: bool,
    pub check_resources: bool,
}

#[spacetimedb::reducer]
pub fn resource_spawn_scheduled(ctx: &ReducerContext, timer: ResourceSpawnTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(&ctx)?;
    if ResourceState::spawn(
        ctx,
        timer.entity_id,
        timer.resource_id,
        timer.coordinates,
        timer.direction_index,
        timer.health,
        timer.check_buildings,
        timer.check_resources,
    )
    .is_err()
    {
        return Err("Unable to spawn resource".into());
    }
    Ok(())
}

impl ResourceState {
    pub fn distance_to(&self, ctx: &ReducerContext, coordinates: SmallHexTile) -> i32 {
        coordinates.distance_to_footprint(self.footprint(ctx, game_state::game_state_filters::coordinates(ctx, self.entity_id)))
    }

    pub fn footprint(&self, ctx: &ReducerContext, center: SmallHexTile) -> Vec<(SmallHexTile, FootprintType)> {
        if let Some(resource_desc) = ctx.db.resource_desc().id().find(&self.resource_id) {
            return resource_desc.get_footprint(&center, self.direction_index);
        }

        Vec::new()
    }

    pub fn get_at_location(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<ResourceState> {
        LocationState::select_all(ctx, coordinates)
            .filter_map(|ls| ctx.db.resource_state().entity_id().find(&ls.entity_id))
            .next()
    }

    /// Insert a single `ResourceDeposit` and update the corresponding `ResourceCount`.
    ///
    /// For batch inserts, this will be less efficient than reading the `ResourceCount` into WASM memory,
    /// incrementing it for each new deposit,
    /// then inserting the updated `ResourceCount` once at the end.
    pub fn insert_one(ctx: &ReducerContext, resource: ResourceState) -> Result<(), String> {
        let resource_id = resource.resource_id;

        ctx.db.resource_state().try_insert(resource)?;

        if let Some(mut count) = ctx.db.resource_count().resource_id().find(resource_id) {
            count.num_in_world += 1;
            ctx.db.resource_count().resource_id().update(count);
        } else {
            ctx.db.resource_count().insert(ResourceCount {
                resource_id,
                num_in_world: 0,
            });
        };
        Ok(())
    }

    /// Delete a single `ResourceDeposit` from the world, and update the corresponding `ResourceCount`.
    ///
    /// Returns `true` if a `ResourceDeposit` with `entity_id` previously existed and was deleted,
    /// or `false` if no such `ResourceDeposit` exists.
    ///
    /// If a `ResourceDeposit` with `entity_id` exists in the world,
    /// it *must* have `resource_id` as its resource type.
    /// Failure to uphold this constraint will result in inconsistent `ResourceCount`s.
    ///
    /// If the `resource_id` is not already in scope, use [`ResourceState::delete_one_by_entity_id`].
    ///
    /// For batch deletes, this will be less efficient than reading the `ResourceCount` into WASM memory,
    /// decrementing it for each removed deposit,
    /// then inserting the updated `ResourceCount` once at the end.
    pub fn delete_one_by_entity_id_with_type(ctx: &ReducerContext, entity_id: &u64, resource_id: i32) -> bool {
        // Schedule a respawn within the same block
        if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
            let coord = location.coordinates();
            if coord.dimension == dimensions::OVERWORLD {
                if let Some(resource) = ctx.db.resource_state().entity_id().find(entity_id) {
                    let resource_id = resource.resource_id;
                    let resource_desc = ctx.db.resource_desc().id().find(&resource_id).unwrap();
                    if resource_desc.scheduled_respawn_time > 0.0 {
                        // Only spawn single-resource clumps
                        if let Some(resource_clump_desc) = ctx.db.single_resource_to_clump_desc().resource_id().find(&resource_id) {
                            let chunk_index = location.chunk_index;
                            ctx.db
                                .respawn_resource_in_chunk_timer()
                                .try_insert(RespawnResourceInChunkTimer {
                                    scheduled_at: now_plus_secs_f32(resource_desc.scheduled_respawn_time, ctx.timestamp),
                                    scheduled_id: 0,
                                    chunk_index,
                                    resource_clump_id: resource_clump_desc.clump_id,
                                    coord,
                                })
                                .ok()
                                .unwrap();
                        }
                    }
                }
            }
        }

        if ctx.db.resource_state().entity_id().delete(entity_id) {
            let mut count = ctx
                .db
                .resource_count()
                .resource_id()
                .find(&resource_id)
                .unwrap_or_else(|| panic!("No ResourceCount for resource_id: {}", resource_id));
            count.num_in_world -= 1;
            ctx.db.resource_count().resource_id().update(count);
            true
        } else {
            false
        }
    }

    /// Delete a single `ResourceDeposit` from the world, and update the corresponding `ResourceCount`.
    ///
    /// Returns `true` if a `ResourceDeposit` with `entity_id` previously existed and was deleted,
    /// or `false` if no such `ResourceDeposit` exists.
    ///
    /// This method must first do a lookup with `ResourceState::filter_by_entity_id`
    /// to determine the resource's `resource_id` type.
    /// If you already have access to the `resource_id`,
    /// instead use [`ResourceState::delete_one_by_entity_id_with_type`].
    ///
    /// For batch deletes, this will be less efficient than reading the `ResourceCount` into WASM memory,
    /// decrementing it for each removed deposit,
    /// then inserting the updated `ResourceCount` once at the end.
    pub fn delete_one_by_entity_id(ctx: &ReducerContext, entity_id: &u64) -> bool {
        if let Some(row) = ctx.db.resource_state().entity_id().find(entity_id) {
            Self::delete_one_by_entity_id_with_type(ctx, entity_id, row.resource_id)
        } else {
            false
        }
    }

    // DAB Note: Use this function instead once queries are optimized
    /*
    pub fn get_at_location(coordinates: &SmallHexTile) -> Option<ResourceDeposit> {
        LocationState::select_all(coordinates)
            .filter_map( |location| ctx.db.resource_state().entity_id().find(&location.entity_id) )
            .next()
    }
    */

    pub fn despawn_self(&self, ctx: &ReducerContext) -> bool {
        let deposit_entity_id = self.entity_id;
        let deposit_resource_id = self.resource_id;
        Self::despawn(ctx, deposit_entity_id, deposit_resource_id)
    }

    pub fn despawn(ctx: &ReducerContext, deposit_entity_id: u64, deposit_resource_id: i32) -> bool {
        // Despawn the resource right away. Falling resources on client will be purely cosmetic and will be handled on client.
        // As far as server is concerned, the falling tree no longer exists.

        // Growing resources are player made and are not part of the eco-system.
        if ctx.db.growth_state().entity_id().find(&deposit_entity_id).is_some()
            || ctx.db.resource_growth_timer().entity_id().find(&deposit_entity_id).is_some()
        {
            ctx.db.resource_state().entity_id().delete(&deposit_entity_id);
            delete_resource_growth_timer(ctx, deposit_entity_id);
        } else if deposit_resource_id != 0 && !Self::delete_one_by_entity_id_with_type(ctx, &deposit_entity_id, deposit_resource_id) {
            log::error!(
                "Resource {} of type {} was deleted but didn't exist.",
                deposit_entity_id,
                deposit_resource_id
            );
            return false;
        }

        crumb_trail_clean_up_agent::clean_up_if_final_prize_resource_deleted(ctx, deposit_entity_id);
        ctx.db.light_source_state().entity_id().delete(&deposit_entity_id);
        delete_footprint(ctx, deposit_entity_id);
        AttachedHerdsState::delete(ctx, deposit_entity_id);
        delete_entity(ctx, deposit_entity_id);
        true
    }

    pub fn despawn_growing(&self, ctx: &ReducerContext) -> bool {
        if !ctx.db.resource_state().entity_id().delete(&self.entity_id) {
            return false;
        }
        ctx.db.light_source_state().entity_id().delete(&self.entity_id);
        delete_footprint(ctx, self.entity_id);
        AttachedHerdsState::delete(ctx, self.entity_id);
        ctx.db.distant_visible_entity().entity_id().delete(&self.entity_id);
        ctx.db.growth_state().entity_id().delete(&self.entity_id);
        ctx.db.resource_health_state().entity_id().delete(&self.entity_id);
        ctx.db.location_state().entity_id().delete(&self.entity_id);
        true
    }

    pub fn produce_offspawn(ctx: &ReducerContext, resource_id: i32, coord: SmallHexTile, deposit_direction: i32) {
        // Respawn resource right away since it's likely a resource that undergoes a state change.
        // We assume the footprint will be the same or included within the original shape.
        let resource_desc = ctx.db.resource_desc().id().find(&resource_id).unwrap();
        let respawn_resource_id = resource_desc.on_destroy_yield_resource_id;
        let should_spawn = if resource_desc.on_destroy_yield_resource_chance >= 1.0 {
            true
        } else if resource_desc.on_destroy_yield_resource_chance <= 0.0 {
            false
        } else {
            ctx.rng().gen_range(0.0..=1.0) <= resource_desc.on_destroy_yield_resource_chance
        };

        if respawn_resource_id != 0 && should_spawn {
            let _ = Self::spawn_in_radius_band_with_fallback(
                ctx,
                respawn_resource_id,
                coord,
                deposit_direction,
                resource_desc.on_destroy_yield_resource_min_radius,
                resource_desc.on_destroy_yield_resource_max_radius,
            );
        }
    }

    pub fn spawn_in_radius_band_with_fallback(
        ctx: &ReducerContext,
        resource_id: i32,
        center: SmallHexTile,
        direction_index: i32,
        min_radius: i32,
        max_radius: i32,
    ) -> bool {
        if resource_id == 0 {
            return false;
        }

        let resource_desc = ctx.db.resource_desc().id().find(&resource_id).unwrap();
        let resource_health = resource_desc.max_health;
        let min_radius = min_radius.max(0);
        let max_radius = max_radius.max(min_radius);
        let max_attempts = ctx
            .db
            .parameters_desc()
            .version()
            .find(&0)
            .map(|params| params.auto_respawn_attempts.max(0) as usize);
        let mut terrain_cache = TerrainChunkCache::empty();

        if min_radius == 0 && max_radius == 0 {
            return Self::spawn(ctx, None, resource_id, center, direction_index, resource_health, false, false).is_ok();
        }

        let mut attempts = SmallHexTile::tile_count_between_radius(min_radius, max_radius);
        if let Some(max_attempts) = max_attempts {
            attempts = attempts.min(max_attempts as u64);
        }
        let mut sampled = HashSet::new();

        for _ in 0..attempts {
            let candidate = SmallHexTile::random_tile_between_radius(ctx, center, min_radius, max_radius, &mut sampled);
            if candidate != center
                && !resources_regen::is_valid_single_resource_spawn(ctx, &mut terrain_cache, &resource_desc, candidate, direction_index)
            {
                continue;
            }

            if Self::spawn(ctx, None, resource_id, candidate, direction_index, resource_health, false, false).is_ok() {
                return true;
            }
        }

        Self::spawn(ctx, None, resource_id, center, direction_index, resource_health, false, false).is_ok()
    }

    pub fn schedule_resource_spawn(ctx: &ReducerContext, resource_id: i32, coordinates: SmallHexTileMessage, direction_index: i32) {
        let health = ctx.db.resource_desc().id().find(&resource_id).unwrap().max_health;
        let _ = ctx.db.resource_spawn_timer().try_insert(ResourceSpawnTimer {
            entity_id: None,
            resource_id,
            coordinates,
            direction_index,
            health,
            check_buildings: false,
            check_resources: false,
            scheduled_id: 0,
            scheduled_at: ctx.timestamp.into(),
        });
    }

    pub fn spawn_from_construction_site(
        ctx: &ReducerContext,
        entity_id: u64,
        resource: &ResourceDesc,
        coordinates: SmallHexTile,
        direction_index: i32,
        health: i32,
    ) -> Result<(), String> {
        let deposit_state = ResourceState {
            entity_id,
            resource_id: resource.id,
            direction_index,
        };

        // We do not want to use insert_one so that type of resource isn't added to the eco-system
        // and considered for the resources regen
        ctx.db.resource_state().try_insert(deposit_state)?;
        Self::insert_light_source_state(ctx, entity_id, resource.light_radius);

        let health_state = ResourceHealthState { entity_id, health };

        ctx.db.resource_health_state().try_insert(health_state)?;

        // Location and footprints already exist
        footprint_helpers::update_footprint_after_resource_completion(ctx, entity_id, coordinates, direction_index, resource);

        Self::add_growth_timer(ctx, entity_id, resource.id);
        Self::create_distant_visibile_resource(ctx, &resource, entity_id, coordinates);

        Ok(())
    }

    pub fn spawn(
        ctx: &ReducerContext,
        entity_id: Option<u64>,
        resource_id: i32,
        coordinates: SmallHexTile,
        direction_index: i32,
        health: i32,
        check_buildings: bool,
        check_resources: bool,
    ) -> Result<u64, String> {
        //log::info!(
        //    "Spawn Resource id {} at ({},{},{})",
        //    resource_id,
        //    coordinates.x,
        //    coordinates.z,
        //    coordinates.dimension,
        //);
        let offset = coordinates.to_offset_coordinates();

        let entity_id = entity_id.unwrap_or(game_state::create_entity(ctx));

        let deposit_state = ResourceState {
            entity_id,
            resource_id,
            direction_index,
        };
        let resource_desc = unwrap_or_err!(ctx.db.resource_desc().id().find(&deposit_state.resource_id), "Unknown resource");

        if check_buildings || check_resources {
            // destroy all resources under the footprint (ignore priority, the priority is used in the agent)
            let resource_footprints = &resource_desc.footprint;
            let mut resources_to_delete: HashSet<u64> = HashSet::new();

            for delta in resource_footprints
                .into_iter()
                .filter(|f| f.footprint_type != FootprintType::Perimeter)
            {
                let footprint_coordinates = (SmallHexTile {
                    x: coordinates.x + delta.x,
                    z: coordinates.z + delta.z,
                    dimension: coordinates.dimension,
                })
                .rotate_around(&coordinates, (deposit_state.direction_index as i32) / 2);

                let footprints = FootprintTileState::get_at_location(ctx, &footprint_coordinates);
                for footprint in footprints {
                    if check_buildings && ctx.db.building_state().entity_id().find(&footprint.owner_entity_id).is_some() {
                        return Err("Cannot spawn a resource over a building".into());
                    }
                    if check_resources && ctx.db.resource_state().entity_id().find(&footprint.owner_entity_id).is_some() {
                        resources_to_delete.insert(footprint.owner_entity_id);
                    }
                }
            }

            // delete previous resources in place
            for deposit_entity_id in resources_to_delete.iter() {
                ResourceState::despawn(
                    ctx,
                    *deposit_entity_id,
                    ctx.db.resource_state().entity_id().find(deposit_entity_id).unwrap().resource_id,
                );
            }
        }

        game_state::insert_location(ctx, entity_id, offset);

        Self::insert_one(ctx, deposit_state)?;
        Self::insert_light_source_state(ctx, entity_id, resource_desc.light_radius);

        Self::add_growth_timer(ctx, entity_id, resource_id);

        let health_state = ResourceHealthState { entity_id, health };

        ctx.db.resource_health_state().try_insert(health_state)?;
        Self::create_distant_visibile_resource(ctx, &resource_desc, entity_id, coordinates);

        if resource_desc.footprint.is_empty() {
        } else {
            // create footprint
            footprint_helpers::create_resource_footprint(ctx, entity_id, &resource_desc, direction_index);
            // TODO: flatten footprint
        }
        if resource_desc.enemy_params_id.len() > 0 {
            HerdState::attach(ctx, entity_id, resource_desc.enemy_params_id, offset);
        }

        return Ok(entity_id);
    }

    fn add_growth_timer(ctx: &ReducerContext, entity_id: u64, resource_id: i32) {
        // Schedule growth if required.
        let growth = ctx.db.resource_growth_recipe_desc().resource_id().filter(resource_id).next();
        // For now we assume only 1 entry per resource. Will add some logic if multiple recipes can target the same resource id.
        if let Some(growth) = growth {
            insert_resource_growth_timer(ctx, entity_id, &growth).unwrap();
        }
    }

    fn create_distant_visibile_resource(ctx: &ReducerContext, resource_desc: &ResourceDesc, entity_id: u64, coordinates: SmallHexTile) {
        if ctx
            .db
            .distant_visible_entity_desc()
            .description_id()
            .filter(resource_desc.id)
            .any(|x| x.entity_type == EntityType::Resource)
        {
            ctx.db.distant_visible_entity().insert(DistantVisibleEntity {
                entity_id: entity_id,
                chunk_index: coordinates.chunk_coordinates().chunk_index(),
            });
        }
    }

    fn insert_light_source_state(ctx: &ReducerContext, entity_id: u64, light_radius: i32) {
        if light_radius > 0 {
            ctx.db.light_source_state().insert(LightSourceState {
                entity_id,
                radius: light_radius as f32,
            });
        }
    }
}
