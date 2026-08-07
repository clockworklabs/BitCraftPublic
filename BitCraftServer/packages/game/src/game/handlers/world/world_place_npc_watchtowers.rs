use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        claim_helper,
        coordinates::*,
        game_state::{self, create_entity, game_state_filters, insert_location},
        handlers::authentication::has_role,
        npc_empire::NPC_EMPIRE_DEFAULT_NAME,
        reducer_helpers::{
            building_helpers::{
                create_building_claim, create_building_component, create_building_footprint, create_building_spawns, delete_building,
            },
            world_placement_helpers::{validate_dimension_rules, verify_or_prepare_footprint},
        },
        terrain_chunk::TerrainChunkCache,
    },
    inter_module,
    messages::{
        authentication::Role,
        components::{building_state, BuildingNicknameState},
        empire_shared::{empire_state, EmpireOwnerType},
        inter_module::{MessageContentsV5, NpcPlaceWatchtowersMsg, NpcWatchtowerPlacement},
        static_data::{building_desc, construction_recipe_desc},
        util::OffsetCoordinatesSmallMessage,
        world::{world_entity_placement_results, WorldEntityPlacement, WorldEntityPlacementResults, WorldPlacementType},
    },
};

const MIN_CLAIM_TOTEM_SMALL_TILE_DISTANCE: i32 = 80;

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn world_place_npc_watchtowers(
    ctx: &ReducerContext,
    watchtower_positions: Vec<OffsetCoordinatesSmallMessage>,
    watchtower_chunk_indexes: Vec<Vec<u64>>,
    energy: i32,
    upkeep: i32,
    building_desc_id: i32,
    biomes: Vec<i32>,
    dry_run: bool,
    log_results: bool,
    clear_and_level_ground: bool,
    ignore_biomes: bool,
    ignore_claims: bool,
    ignore_dimension_rules: bool,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    if watchtower_positions.len() != watchtower_chunk_indexes.len() {
        return Err(format!(
            "Position count ({}) must match chunk index list count ({})",
            watchtower_positions.len(),
            watchtower_chunk_indexes.len()
        ));
    }

    let building_desc = ctx
        .db
        .building_desc()
        .id()
        .find(&building_desc_id)
        .ok_or_else(|| format!("Building desc {} not found", building_desc_id))?;

    // Validate no duplicate chunks across watchtowers
    let mut all_chunks = std::collections::HashSet::new();
    for chunks in &watchtower_chunk_indexes {
        for &chunk_idx in chunks {
            if !all_chunks.insert(chunk_idx) {
                return Err(format!("Duplicate chunk index {} in watchtower assignments", chunk_idx));
            }
        }
    }

    for row in ctx.db.world_entity_placement_results().iter() {
        ctx.db.world_entity_placement_results().delete(row);
    }

    let recipe = ctx
        .db
        .construction_recipe_desc()
        .building_description_id()
        .filter(building_desc_id)
        .next();

    let mut terrain_cache = TerrainChunkCache::empty();

    // Look up actual NPC empire name (set by user in the UI, not the hardcoded default)
    let npc_empire_name = ctx
        .db
        .empire_state()
        .iter()
        .find(|e| e.owner_type == EmpireOwnerType::Npc)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| NPC_EMPIRE_DEFAULT_NAME.to_string());

    // Create building entities and collect placement data for global
    let mut placements: Vec<NpcWatchtowerPlacement> = Vec::with_capacity(watchtower_positions.len());
    let mut placements_out: Vec<WorldEntityPlacement> = if log_results {
        Vec::with_capacity(watchtower_positions.len())
    } else {
        Vec::new()
    };

    for (i, position) in watchtower_positions.iter().enumerate() {
        let coords = SmallHexTile::from(*position);

        // --- Validation (mirrors world_place_building) ---

        // Footprint / biome / water validation
        let footprint = match verify_or_prepare_footprint(
            ctx,
            &mut terrain_cache,
            coords,
            0, // facing direction (watchtowers are non-directional)
            &building_desc,
            &biomes,
            clear_and_level_ground,
            dry_run,
            ignore_biomes,
        ) {
            Ok(fp) => fp,
            Err(e) => {
                log::info!("world_place_npc_watchtowers: footprint invalid at {:?}: {}", position, e);
                continue;
            }
        };

        // Dimension rules validation
        if let Some(ref recipe) = recipe {
            if let Err(e) = validate_dimension_rules(ctx, coords, recipe.required_interior_tier, ignore_dimension_rules) {
                log::info!("world_place_npc_watchtowers: dimension invalid: {}", e);
                continue;
            }
        }

        // Claim overlap validation
        if !ignore_claims {
            let existing_claims = claim_helper::get_partial_claims_under_footprint(ctx, &footprint);
            if !existing_claims.is_empty() {
                log::info!("world_place_npc_watchtowers: footprint overlaps existing claims");
                continue;
            }
        }

        // Watchtower-specific: minimum distance from settlement totems
        if game_state_filters::any_claim_totems_in_radius(ctx, coords, MIN_CLAIM_TOTEM_SMALL_TILE_DISTANCE) {
            log::info!("world_place_npc_watchtowers: too close to settlement totem");
            continue;
        }

        // --- Dry run: collect placement data only ---
        if dry_run {
            if log_results {
                placements_out.push(WorldEntityPlacement {
                    entity_id: 0,
                    coordinates: *position,
                    prototype_id: building_desc_id,
                    placement_type: WorldPlacementType::Building,
                });
            }
            continue;
        }

        // --- Actual placement ---
        let entity_id = create_entity(ctx);
        let offset: OffsetCoordinatesSmall = *position;

        insert_location(ctx, entity_id, offset);
        create_building_component(ctx, 0, entity_id, 0, &building_desc, 0);
        create_building_footprint(ctx, entity_id, 0, &building_desc, &None);
        create_building_spawns(ctx, entity_id);
        create_building_claim(ctx, entity_id, true)?;

        BuildingNicknameState::insert_shared(
            ctx,
            BuildingNicknameState {
                entity_id,
                nickname: format!("{}'s {}", npc_empire_name, building_desc.name),
            },
            inter_module::InterModuleDestination::Global,
        );

        placements.push(NpcWatchtowerPlacement {
            building_entity_id: entity_id,
            location: *position,
            chunk_indexes: watchtower_chunk_indexes[i].clone(),
        });

        if log_results {
            placements_out.push(WorldEntityPlacement {
                entity_id,
                coordinates: *position,
                prototype_id: building_desc_id,
                placement_type: WorldPlacementType::Building,
            });
        }
    }

    if !dry_run {
        log::info!("Created {} watchtower buildings in region, sending to global", placements.len());

        // Send placement data to global module for empire node + chunk creation
        inter_module::send_inter_module_message(
            ctx,
            MessageContentsV5::NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg {
                watchtowers: placements,
                energy,
                upkeep,
            }),
            inter_module::InterModuleDestination::Global,
        );
    } else {
        log::info!(
            "Dry run: would place {} watchtower buildings (after validation)",
            placements_out.len()
        );
    }

    if log_results {
        let row = WorldEntityPlacementResults {
            entity_id: create_entity(ctx),
            timestamp: game_state::unix(ctx.timestamp),
            placements: placements_out,
            dry_run,
            add_to_resources_log: false,
        };
        let _ = ctx.db.world_entity_placement_results().try_insert(row);
    }

    Ok(())
}

/// Deletes specific watchtower building entities on this region.
/// The client collects entity IDs from EmpireNodeState and groups them by region.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn world_clear_npc_watchtowers(ctx: &ReducerContext, building_entity_ids: Vec<u64>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut deleted = 0u64;
    for entity_id in &building_entity_ids {
        if ctx.db.building_state().entity_id().find(entity_id).is_some() {
            delete_building(ctx, 0, *entity_id, None, true, false);
            deleted += 1;
        } else {
            log::warn!(
                "world_clear_npc_watchtowers: building {} not found on this region, skipping",
                entity_id
            );
        }
    }

    log::info!(
        "Deleted {}/{} NPC watchtower buildings on this region",
        deleted,
        building_entity_ids.len()
    );
    Ok(())
}
