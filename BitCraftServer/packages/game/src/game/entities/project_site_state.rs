use spacetimedb::{log, ReducerContext, Table};

use super::location::LocationState;
pub use crate::game::coordinates::*;
use crate::game::game_state;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::dropped_inventory_state;
use crate::{deployable_state_v2, resource_state, unwrap_or_err, Biome};
use crate::{
    game::{claim_helper, game_state::game_state_filters},
    messages::{
        components::{FootprintTileState, PavedTileState, ProjectSiteState, TerrainCell},
        static_data::*,
    },
};

impl ProjectSiteState {
    pub fn distance_to(&self, ctx: &ReducerContext, coordinates: SmallHexTile) -> i32 {
        coordinates.distance_to_footprint(self.footprint(ctx, game_state::game_state_filters::coordinates(ctx, self.entity_id)))
    }

    pub fn footprint(&self, ctx: &ReducerContext, center: SmallHexTile) -> Vec<(SmallHexTile, FootprintType)> {
        if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(&self.construction_recipe_id) {
            if let Some(building_desc) = ctx.db.building_desc().id().find(&recipe.building_description_id) {
                return building_desc.get_footprint(&center, self.direction);
            }
        } else if let Some(recipe) = ctx
            .db
            .resource_placement_recipe_desc()
            .id()
            .find(&self.resource_placement_recipe_id)
        {
            if let Some(resource_desc) = ctx.db.resource_desc().id().find(recipe.resource_description_id) {
                return resource_desc.get_footprint(&center, self.direction);
            }
        }

        Vec::new()
    }

    pub fn validate_placement(
        ctx: &ReducerContext,
        terrain_cache: &mut TerrainChunkCache,
        coordinates: SmallHexTile,
        player_entity_id: u64,
        footprint: &Vec<(SmallHexTile, FootprintType)>,
        required_paving: i32,
        player_must_be_neighbor: bool,
        original_claim_id: u64,
        ignore_building_entity_id: Option<u64>,
        ignore_player_placement: bool,
    ) -> Result<(), String> {
        Self::validate_placement_with_platform(
            ctx,
            terrain_cache,
            coordinates,
            player_entity_id,
            footprint,
            required_paving,
            player_must_be_neighbor,
            original_claim_id,
            ignore_building_entity_id,
            ignore_player_placement,
            None,
        )
    }

    /// Checks that a terrain cell can hold the given floating platform (water depth range and allowed biomes).
    pub fn validate_floating_platform_cell(cell: &TerrainCell, platform: &FloatingPlatformDesc) -> Result<(), String> {
        let depth = cell.water_depth();
        if !cell.is_submerged() || depth < platform.min_water_depth {
            return Err("The water is too shallow here.".into());
        }
        if depth > platform.max_water_depth {
            return Err("The water is too deep here.".into());
        }
        if !platform.allowed_biomes.is_empty() && !platform.allowed_biomes.iter().any(|b| *b as i32 == cell.biome()) {
            return Err("Can't build this in this biome.".into());
        }
        Ok(())
    }

    pub fn validate_placement_with_platform(
        ctx: &ReducerContext,
        terrain_cache: &mut TerrainChunkCache,
        coordinates: SmallHexTile,
        player_entity_id: u64,
        footprint: &Vec<(SmallHexTile, FootprintType)>,
        required_paving: i32,
        player_must_be_neighbor: bool,
        original_claim_id: u64,
        ignore_building_entity_id: Option<u64>,
        ignore_player_placement: bool,
        floating_platform: Option<&FloatingPlatformDesc>,
    ) -> Result<(), String> {
        if !ignore_player_placement {
            if coordinates.distance_to(game_state_filters::coordinates_float(ctx, player_entity_id).into())
                > ctx.db.parameters_desc().version().find(&0).unwrap().max_build_range
            {
                return Err("Too far".into());
            }
        }

        let close_enemies: Vec<SmallHexTile> = game_state_filters::enemies_in_radius(ctx, coordinates, 5)
            .map(|(_, coord)| coord)
            .collect();

        let main_cell = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
            "Invalid location"
        );
        let main_elevation = main_cell.elevation;
        let main_water_level = main_cell.water_level;

        let mut player_is_neighbor = false;

        if original_claim_id != 0 {
            if claim_helper::get_claim_under_footprint(ctx, &footprint) != original_claim_id {
                return Err("Can only move a building within its own claim.".into());
            }
        }

        let player_coord_opt: Option<FloatHexTile> = if !ignore_player_placement {
            Some(game_state_filters::coordinates_float(ctx, player_entity_id).into())
        } else {
            None
        };

        for (coords, footprint_type) in footprint {
            let existing_footprints = FootprintTileState::get_at_location(ctx, &coords);
            for f in existing_footprints {
                let resource = ctx.db.resource_state().entity_id().find(&f.owner_entity_id);
                if resource.is_none() && !FootprintTile::is_compatible(&f.footprint_type, footprint_type) {
                    let is_ignored = match ignore_building_entity_id {
                        Some(building_entity_id) => building_entity_id == f.owner_entity_id,
                        None => false,
                    };

                    if !is_ignored {
                        return Err("Can't build on top of another building".into());
                    }
                }
                if let Some(deposit) = resource {
                    if !ctx.db.resource_desc().id().find(&deposit.resource_id).unwrap().flattenable {
                        return Err("Can't build on top of this resource.".into());
                    }
                }
            }

            if required_paving >= 0 && *footprint_type != FootprintType::Perimeter {
                let valid = match PavedTileState::get_at_location(ctx, &coords) {
                    Some(tile) => ctx.db.paving_tile_desc().id().find(&tile.tile_type_id).unwrap().tier >= required_paving,
                    None => false,
                };
                if !valid {
                    if required_paving == 0 {
                        return Err("This building requires paving!".into());
                    } else {
                        return Err(format!("This building requires tier {{0}} paving!|~{}", required_paving).into());
                    }
                }
            }

            let terrain_target = match terrain_cache.get_terrain_cell(ctx, &(*coords).parent_large_tile()) {
                Some(e) => e,
                None => return Err("Invalid footprint.".into()),
            };

            if *footprint_type != FootprintType::Perimeter && floating_platform.is_some() {
                // Floating platforms must be entirely over water of the right depth, on a flat water surface
                let platform = floating_platform.unwrap();
                for cell_coords in coords.get_terrain_coordinates() {
                    let cell = unwrap_or_err!(terrain_cache.get_terrain_cell(ctx, &cell_coords), "Invalid footprint.");
                    Self::validate_floating_platform_cell(&cell, platform)?;
                    if cell.water_level != main_water_level {
                        return Err("Can only build on a flat water surface.".into());
                    }
                }
            } else if *footprint_type != FootprintType::Perimeter {
                if let Some((_, platform)) = game_state_filters::floating_platform_at_coordinates(ctx, *coords) {
                    if !platform.supports_buildings {
                        return Err("Can't build on this platform.".into());
                    }
                }

                if !game_state_filters::is_interior_tile_walkable(ctx, *coords) {
                    return Err("Can't build outside interior".into());
                }

                if coords.is_corner() && !game_state_filters::is_flat_corner(ctx, terrain_cache, *coords) {
                    return Err("Can only build on a flat surface.".into());
                }

                if terrain_target.elevation != main_elevation {
                    return Err("Can only build on a flat surface".into());
                }

                if game_state_filters::is_submerged(ctx, terrain_cache, *coords) {
                    return Err("Can't build over water.".into());
                }
            }

            if close_enemies.contains(coords) {
                return Err("Can't build on top of an enemy".into());
            }

            if let Some(player_coord) = player_coord_opt {
                if player_coord.distance_to((*coords).into()) <= 1.5 {
                    player_is_neighbor = true;
                }
            }

            let entities_on_tile = LocationState::select_all(ctx, coords)
                .map(|a| {
                    (
                        ctx.db.deployable_state_v2().entity_id().find(&a.entity_id),
                        ctx.db.dropped_inventory_state().entity_id().find(&a.entity_id),
                    )
                })
                .find(|a| a.0.is_some() || a.1.is_some());

            if let Some(pair) = entities_on_tile {
                if pair.0.is_some() {
                    return Err("Can't build on top of a deployable".into());
                }
                log::warn!("Project site was built on top of a dropped inventory");
            }
        }

        if !ignore_player_placement && player_must_be_neighbor && !player_is_neighbor {
            return Err("Need to be next to a building to build it.".into());
        }

        Ok(())
    }

    pub fn validate_building_placement(
        ctx: &ReducerContext,
        terrain_cache: &mut TerrainChunkCache,
        coordinates: SmallHexTile,
        facing_direction: HexDirection,
        player_entity_id: u64,
        building: &BuildingDesc,
        player_must_be_neighbor: bool,
        original_claim_id: u64,
        ignore_building_entity_id: Option<u64>,
    ) -> Result<(), String> {
        let building_description = ctx.db.building_desc().id().find(&building.id).unwrap();
        if building_description.has_category(ctx, BuildingCategory::TerrraformingBase) {
            if !coordinates.is_center() {
                return Err("Terraforming base can only be built in the center of a terrain cell".into());
            }
        };

        if building_description.has_category(ctx, BuildingCategory::Elevator) {
            Self::is_valid_elevator_placement(ctx, terrain_cache, coordinates, facing_direction)?
        }

        let floating_platform = ctx.db.floating_platform_desc().building_id().find(&building.id);
        if floating_platform.is_some() {
            if coordinates.dimension != crate::game::dimensions::OVERWORLD {
                return Err("Floating platforms can only be built outside".into());
            }
            if !coordinates.is_center() {
                return Err("Floating platforms can only be built in the center of a terrain cell".into());
            }
        }

        // Don't allow placing close to prohibited biomes
        for biome in ctx.db.biome_desc().disallow_player_build().filter(true) {
            let terrain_target = unwrap_or_err!(
                terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
                "Invalid footprint"
            );

            if terrain_target.biome_percentage(Biome::to_enum(biome.biome_type)) > 0f32 {
                return Err("Can't build in this biome".into());
            }
        }

        let footprint = building.get_footprint(&coordinates, facing_direction as i32);
        let required_paving = match ctx
            .db
            .construction_recipe_desc()
            .building_description_id()
            .filter(building.id)
            .next()
        {
            Some(recipe) => recipe.required_paving_tier,
            None => -1,
        };
        Self::validate_placement_with_platform(
            ctx,
            terrain_cache,
            coordinates,
            player_entity_id,
            &footprint,
            required_paving,
            player_must_be_neighbor,
            original_claim_id,
            ignore_building_entity_id,
            false,
            floating_platform.as_ref(),
        )
    }

    fn is_valid_elevator_placement(
        ctx: &ReducerContext,
        terrain_cache: &mut TerrainChunkCache,
        coordinates: SmallHexTile,
        facing_direction: HexDirection,
    ) -> Result<(), String> {
        let neighbor = coordinates.neighbor(facing_direction);

        if neighbor.is_corner() {
            return Err("Elevators must be built facing a climbable cliff".into());
        }

        let terrain_cell = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
            "Invalid footprint"
        );

        let neighbor_terrain_cell = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &neighbor.parent_large_tile()),
            "Invalid footprint"
        );
        let elevation_delta = neighbor_terrain_cell.elevation - terrain_cell.elevation;

        if !ctx
            .db
            .climb_requirement_desc()
            .iter()
            .any(|a| elevation_delta >= a.min_elevation && elevation_delta <= a.max_elevation)
        {
            return Err("Elevators must be built facing a climbable cliff".into());
        }

        Ok(())
    }

    pub fn validate_resource_placement(
        ctx: &ReducerContext,
        recipe: &ResourcePlacementRecipeDesc,
        terrain_cache: &mut TerrainChunkCache,
        coordinates: SmallHexTile,
    ) -> Result<(), String> {
        if recipe.required_biomes.len() == 0 {
            return Ok(());
        }

        let terrain_cell = unwrap_or_err!(
            terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
            "Invalid footprint"
        );

        if recipe.required_biomes.iter().all(|x| terrain_cell.biome_percentage(*x) == 0f32) {
            return Err("Can't be placed in this biome".into());
        }

        Ok(())
    }
}
