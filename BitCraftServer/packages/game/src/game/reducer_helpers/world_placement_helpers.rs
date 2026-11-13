use crate::game::coordinates::SmallHexTile;
use crate::game::game_state::game_state_filters;
use crate::game::reducer_helpers::footprint_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::*;
use crate::messages::static_data::*;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

/// Validates and optionally prepares a building footprint at the given coordinates.
///
/// Checks biome validity, water/submersion, and either clears+levels ground or
/// validates placement rules (paving, overlaps, etc.) depending on flags.
#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
pub fn verify_or_prepare_footprint(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    coords: SmallHexTile,
    facing_dir_i32: i32,
    building: &BuildingDesc,
    valid_biomes: &[i32],
    clear_and_level_ground: bool,
    dry_run: bool,
    ignore_biome: bool,
) -> Result<Vec<(SmallHexTile, FootprintType)>, String> {
    let terrain_coordinates = coords.parent_large_tile();
    let terrain = match terrain_cache.get_terrain_cell(ctx, &terrain_coordinates) {
        Some(t) => t,
        None => return Err("Terrain cell not found".to_string()),
    };

    if !ignore_biome {
        let mut biome_value = 0.0f32;
        for &valid_biome in valid_biomes.iter() {
            let biome_index = valid_biome as u64;
            for j in 0..4 {
                let biome = (terrain.biomes >> (j * 8)) & 0xFF;
                if (biome as u64) == biome_index {
                    let density = ((terrain.biome_density >> (j * 8)) & 0xFF) as f32;
                    biome_value = density / 128.0;
                    if biome_value > 0.0 {
                        break;
                    }
                }
            }
        }

        if biome_value <= 0.0 {
            return Err("Biome not valid for building".to_string());
        }
    }

    let footprint = building.get_footprint(&coords, facing_dir_i32);

    // Always disallow any footprint tiles that are submerged, regardless of clear_and_level_ground / dry_run.
    for (tile, footprint_type) in &footprint {
        if *footprint_type != FootprintType::Perimeter && game_state_filters::is_submerged(ctx, terrain_cache, *tile) {
            return Err("Can't build over water.".to_string());
        }
    }

    // Path 1: CLEAR AND LEVEL
    if clear_and_level_ground {
        if !dry_run {
            footprint_helpers::clear_and_flatten_terrain_under_footprint(ctx, &footprint, true);
        }
        return Ok(footprint);
    }

    // Path 2: NO CLEAR + LEVEL: validate normally (paving, overlaps, enemies, etc.)
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

    ProjectSiteState::validate_placement(ctx, terrain_cache, coords, 0, &footprint, required_paving, false, 0, None, true)?;

    Ok(footprint)
}

/// Validates that the given coordinates are in an appropriate dimension
/// based on the building's required interior tier.
pub fn validate_dimension_rules(
    ctx: &ReducerContext,
    coords: SmallHexTile,
    required_interior_tier: i32,
    ignore_dimension_rules: bool,
) -> Result<(), String> {
    if ignore_dimension_rules {
        return Ok(());
    }

    let dim = unwrap_or_err!(
        ctx.db.dimension_description_state().dimension_id().find(coords.dimension),
        "Invalid dimension"
    );

    if required_interior_tier == -1 && dim.interior_instance_id != 0 {
        return Err("Can only be built in Overworld".into());
    }
    if required_interior_tier > 0 {
        if dim.interior_instance_id == 0 {
            return Err(format!("Requires Tier {{0}} interior|~{}", required_interior_tier));
        }
        let inst = unwrap_or_err!(
            ctx.db.interior_instance_desc().id().find(dim.interior_instance_id),
            "Missing interior instance"
        );
        if inst.tier < required_interior_tier {
            return Err(format!("Requires Tier {{0}} interior|~{}", required_interior_tier));
        }
    }
    Ok(())
}
