use crate::game::handlers::authentication::has_role;
use crate::game::reducer_helpers::building_helpers::create_building_unsafe;
use crate::game::reducer_helpers::world_placement_helpers::{validate_dimension_rules, verify_or_prepare_footprint};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{claim_helper, coordinates::*, game_state};
use crate::messages::authentication::Role;
use crate::messages::components::*;
use crate::messages::empire_shared::{empire_state, EmpireSettlementState};
use crate::messages::game_util::DimensionType;
use crate::messages::static_data::*;
use crate::messages::util::OffsetCoordinatesSmallMessage;
use crate::messages::world::{
    world_entity_placement_results, WorldEntityPlacement, WorldEntityPlacementResults, WorldPlaceBuildingRequest, WorldPlacementType,
};
use crate::{unwrap_or_err, BuildingInteractionLevel};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn world_place_building(ctx: &ReducerContext, request: WorldPlaceBuildingRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    //always clear results table first
    for row in ctx.db.world_entity_placement_results().iter() {
        ctx.db.world_entity_placement_results().delete(row);
    }

    let recipe = request
        .building_spawn_info
        .construction_recipe_id
        .and_then(|id| ctx.db.construction_recipe_desc().id().find(id));

    let building_desc_id = request.building_spawn_info.building_description_id;

    let building = unwrap_or_err!(ctx.db.building_desc().id().find(building_desc_id), "Building not found for recipe");

    let mut placements_out: Vec<WorldEntityPlacement> = Vec::with_capacity(request.buildings.len());
    let mut terrain_cache = TerrainChunkCache::empty();

    for p in &request.buildings {
        let coords = SmallHexTile::from(p.coordinates);
        let facing_i32 = p.facing_direction;

        let dim = match ctx.db.dimension_description_state().dimension_id().find(coords.dimension) {
            Some(d) => d,
            None => {
                log::info!("world_place_building: invalid dimension {}", coords.dimension);
                continue;
            }
        };

        if dim.dimension_type == DimensionType::AncientRuin || dim.dimension_type == DimensionType::Dungeon {
            log::info!("world_place_building: forbidden dimension type");
            continue;
        }

        let footprint_full = match verify_or_prepare_footprint(
            ctx,
            &mut terrain_cache,
            coords,
            facing_i32,
            &building,
            &request.building_spawn_info.biomes,
            request.clear_and_level_ground,
            request.dry_run,
            request.ignore_biomes,
        ) {
            Ok(fp) => fp,
            Err(e) => {
                log::info!("world_place_building: footprint invalid: {}", e);
                continue;
            }
        };

        if let Some(recipe) = &recipe {
            if let Err(e) = validate_dimension_rules(ctx, coords, recipe.required_interior_tier, request.ignore_dimension_rules) {
                log::info!("world_place_building: {}", e);
                continue;
            }
        }

        let existing_claims = claim_helper::get_partial_claims_under_footprint(ctx, &footprint_full);
        let built_on_existing_claims = !existing_claims.is_empty();

        if !request.ignore_claims && built_on_existing_claims {
            log::info!("world_place_building: footprint overlaps multiple claims");
            continue;
        }

        if !request.ignore_claims {
            if let Some(claim_building) = ctx.db.building_claim_desc().building_id().find(building_desc_id) {
                if claim_building.claim_type == ClaimType::Source {
                    if built_on_existing_claims {
                        return Err("Can't build on an existing claim".into());
                    }

                    claim_helper::can_place_claim_totem(ctx, coords, &claim_building, &mut terrain_cache)?;
                } else if claim_building.claim_type == ClaimType::Extension {
                    return Err("Extension totems are obsolete.".into());
                }
            }
        }

        let is_empire_building = building.build_permission == BuildingInteractionLevel::Empire;
        if is_empire_building && !request.ignore_empire_checks {
            let is_empire_foundry = building.has_category(ctx, BuildingCategory::EmpireFoundry);
            let claim_id = claim_helper::get_claim_under_footprint(ctx, &footprint_full);
            if claim_id == 0 {
                log::info!("world_place_building: empire building must be under an aligned claim");
                continue;
            }
            if let Some(settlement) = EmpireSettlementState::from_claim(ctx, claim_id) {
                if settlement.empire_entity_id == 0 {
                    log::info!("world_place_building: claim not aligned with an empire");
                    continue;
                }
            } else {
                log::info!("world_place_building: claim missing settlement/empire alignment");
                continue;
            }

            if is_empire_foundry {
                let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_id), "Claim state missing");
                if ctx
                    .db
                    .empire_state()
                    .capital_building_entity_id()
                    .find(claim.owner_building_entity_id)
                    .is_none()
                {
                    log::info!("world_place_building: foundry must be built in empire capital");
                    continue;
                }
            }
        }

        if request.dry_run {
            if request.log_results {
                placements_out.push(WorldEntityPlacement {
                    entity_id: 0,
                    coordinates: p.coordinates,
                    prototype_id: building_desc_id,
                    placement_type: WorldPlacementType::Building,
                });
            }
            continue;
        }

        let new_entity_id = game_state::create_entity(ctx);
        if let Err(e) = create_building_unsafe(
            ctx,
            0,
            Some(new_entity_id),
            coords,
            facing_i32,
            building_desc_id,
            recipe.as_ref().map(|r| r.id),
        ) {
            log::info!("world_place_building: create_building_unsafe failed: {}", e);
            continue;
        }

        let off: OffsetCoordinatesSmallMessage = OffsetCoordinatesSmall::from(coords);
        game_state::insert_location(ctx, new_entity_id, off);

        if building.has_category(ctx, BuildingCategory::ClaimTotem) {
            let _ = ctx.db.auto_claim_state().try_insert(AutoClaimState {
                entity_id: new_entity_id,
                owner_entity_id: 0,
            });
        }

        if request.log_results {
            placements_out.push(WorldEntityPlacement {
                entity_id: new_entity_id,
                coordinates: p.coordinates,
                prototype_id: building_desc_id,
                placement_type: WorldPlacementType::Building,
            });
        }
    }

    if request.log_results {
        let row = WorldEntityPlacementResults {
            entity_id: game_state::create_entity(ctx),
            timestamp: game_state::unix(ctx.timestamp),
            placements: placements_out,
            dry_run: request.dry_run,
            add_to_resources_log: false,
        };
        let _ = ctx.db.world_entity_placement_results().try_insert(row);
    }
    Ok(())
}
