use bitcraft_macro::feature_gate;
use crate::game::discovery::Discovery;
use crate::game::entities::location::LocationState;
use crate::game::handlers::empires::empires_shared::validate_empire_build_foundry;
use crate::game::reducer_helpers::building_helpers::create_building_unsafe;
use crate::game::reducer_helpers::floating_platform_helpers;
use crate::game::reducer_helpers::footprint_helpers::{self, create_project_site_footprint};
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{claim_helper, coordinates::*, dimensions};
use crate::game::{game_state, permission_helper};
use crate::messages::action_request::PlayerProjectSitePlaceRequest;
use crate::messages::components::*;
use crate::messages::empire_shared::{empire_player_data_state, empire_state, EmpireSettlementState};
use crate::messages::game_util::{DimensionType, ItemStack};
use crate::messages::static_data::*;
use crate::{
    interior_spawn_random_group_desc, unwrap_or_err, unwrap_or_return, BuildingInteractionLevel, InventoryState, PlayerActionType,
};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate("build")]
pub fn project_site_place(ctx: &ReducerContext, request: PlayerProjectSitePlaceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    for existing_state in ctx.db.user_moderation_state().target_identity().filter(&ctx.sender()) {
        if existing_state.user_moderation_policy == UserModerationPolicy::BlockConstruct && ctx.timestamp < existing_state.expiration_time {
            return Err("Your construction privileges have been suspended".into());
        }
    }

    if ctx.db.mounting_state().entity_id().find(&actor_id).is_some() {
        return Err("Can't place a building while in a deployable.".into());
    }

    let coordinates = SmallHexTile::from(request.coordinates);
    let facing_direction = HexDirection::from(request.facing_direction);

    let mut is_rent_terminal = false;
    let mut footprint = Vec::new();

    let construction_recipe = ctx.db.construction_recipe_desc().id().find(&request.construction_recipe_id);
    let resource_placement_recipe = ctx
        .db
        .resource_placement_recipe_desc()
        .id()
        .find(&request.resource_placement_recipe_id);

    let mut terrain_cache = TerrainChunkCache::empty();

    let mut is_empire_building = false;
    let mut is_empire_foundry = false;
    let mut is_claim_totem = false;
    let mut is_bank = false;
    let mut is_market = false;

    //let mut is_empire_stockpile = false;

    // Validate footprint placement for the project site
    if let Some(ref recipe) = construction_recipe {
        let building = unwrap_or_err!(
            ctx.db.building_desc().id().find(&recipe.building_description_id),
            "Building not found."
        );
        is_empire_foundry = building.has_category(ctx, BuildingCategory::EmpireFoundry);
        is_empire_building = building.build_permission == BuildingInteractionLevel::Empire;
        //is_empire_stockpile = building.has_category(BuildingCategory::Storage) && is_empire_building;
        footprint = building.get_footprint(&coordinates, facing_direction as i32);
        ProjectSiteState::validate_building_placement(
            ctx,
            &mut terrain_cache,
            coordinates,
            facing_direction,
            actor_id,
            &building,
            true,
            0,
            None,
        )?;
        if ctx.db.floating_platform_desc().building_id().find(&building.id).is_some() {
            // Floating platforms are placed from land (or from another platform), not while swimming
            let player_cell = game_state::game_state_filters::coordinates_float(ctx, actor_id).parent_large_tile();
            if terrain_cache.get_terrain_cell(ctx, &player_cell).map_or(false, |c| c.player_should_swim()) {
                return Err("Platforms must be placed from land.".into());
            }
            floating_platform_helpers::validate_platform_limit(ctx, building.id, actor_id)?;
        }
        is_rent_terminal = building.has_category(ctx, BuildingCategory::RentTerminal);
        is_claim_totem = building.has_category(ctx, BuildingCategory::ClaimTotem);

        // It's possible to start two bank/market project sites at the same time
        // However attempting to complete the 2nd one will show the error. Too bad for the lost materials,
        // but this would obvisouly be done on purpose in an attempt to circumvent the limitation.
        is_bank = building.has_category(ctx, BuildingCategory::Bank);
        is_market = building.has_category(ctx, BuildingCategory::TownMarket);
        validate_knowledge(ctx, actor_id, &recipe.required_knowledges)?;
    } else if let Some(ref recipe) = resource_placement_recipe {
        let resource = unwrap_or_err!(
            ctx.db.resource_desc().id().find(&recipe.resource_description_id),
            "Resource not found."
        );
        footprint = resource.get_footprint(&coordinates, facing_direction as i32);
        ProjectSiteState::validate_placement(ctx, &mut terrain_cache, coordinates, actor_id, &footprint, -1, true, 0, None, false)?;

        validate_knowledge(ctx, actor_id, &recipe.required_knowledges)?;
    }

    // Find current dimension and if it can potentially be rented
    if coordinates.dimension != dimensions::OVERWORLD {
        let dimension_description_network = unwrap_or_err!(
            DimensionNetworkState::get(ctx, coordinates.dimension),
            "This interior does not exist"
        );
        // If it's rented, assert that the player is part of the whitelist
        if dimension_description_network.rent_entity_id != 0 {
            let rent = unwrap_or_err!(
                ctx.db.rent_state().entity_id().find(&dimension_description_network.rent_entity_id),
                "This building is rented but the rent doesn't exist."
            );
            if !rent.white_list.contains(&actor_id) {
                return Err("You don't have permission to build here".into());
            }
        } else if is_rent_terminal {
            // Can only build a rent terminal if it's not rented and it's rentable (and if there's no constructed buildings/resources on the floor)
            validate_rentability(ctx, coordinates.dimension, &dimension_description_network)?;
        } else {
            // If there's a rent terminal in construction, we can't build anything.
            let all_locations = LocationState::select_all_in_interior_dimension_iter(ctx, coordinates.dimension);

            let mut all_construction_building_ids =
                all_locations.filter_map(|l| match ctx.db.project_site_state().entity_id().find(&l.entity_id) {
                    Some(c) => Some(
                        ctx.db
                            .construction_recipe_desc()
                            .id()
                            .find(&c.construction_recipe_id)
                            .unwrap()
                            .building_description_id,
                    ),
                    None => None,
                });
            if all_construction_building_ids.any(|id| {
                ctx.db
                    .building_desc()
                    .id()
                    .find(&id)
                    .unwrap()
                    .has_category(ctx, BuildingCategory::RentTerminal)
            }) {
                return Err("Cannot build something in this building while a Rent Terminal is in construction.".into());
            }
        }
    } else if is_rent_terminal {
        return Err("This can only be built inside a rentable building.".into());
    }

    // Claim project site
    let existing_claims_id = claim_helper::get_partial_claims_under_footprint(ctx, &footprint);
    let built_on_existing_claims = existing_claims_id.len() > 0;

    let mut project_owner_id = 0;
    let required_claim_tech_ids = if let Some(ref recipe) = construction_recipe {
        &recipe.required_claim_tech_ids
    } else if let Some(ref recipe) = resource_placement_recipe {
        &recipe.required_claim_tech_ids
    } else {
        &vec![]
    };

    if !PermissionState::can_interact_with_tile(ctx, actor_id, coordinates, Permission::Build) {
        return Err("You don't have permission to build there".into());
    }

    // Can only potentially be owned if a single claim is found under the building (assuming all tiles are covered)
    if existing_claims_id.len() == 1 {
        if !permission_helper::has_permission(ctx, actor_id, coordinates.dimension, existing_claims_id[0], ClaimPermission::Build) {
            return Err("You don't have permission to build there.".into());
        }

        project_owner_id = claim_helper::get_claim_under_footprint(ctx, &footprint);
        if required_claim_tech_ids.len() > 0 {
            let claim_tech = unwrap_or_err!(
                ctx.db.claim_tech_state().entity_id().find(&project_owner_id),
                "This project site needs to be fully under a claim in order to be built"
            );
            for required_claim_tech_id in required_claim_tech_ids {
                // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
                if !claim_tech.has_unlocked_tech(*required_claim_tech_id) {
                    return Err("This claim is missing necessary upgrades for this project".into());
                }
            }
        }
    } else {
        if existing_claims_id.len() > 1 {
            return Err("You cannot build a project site overlapping several claims".into());
        }
        if required_claim_tech_ids.len() != 0 {
            return Err("This project site needs to be fully under a claim in order to be built".into());
        }
    }

    let dimension = unwrap_or_err!(
        ctx.db.dimension_description_state().dimension_id().find(&coordinates.dimension),
        "Invalid dimension"
    );
    if dimension.dimension_type == DimensionType::AncientRuin || dimension.dimension_type == DimensionType::Dungeon {
        return Err("Can't build inside Ancient Ruins".into());
    }

    let mut required_interior_tier = 0;

    // Building-specific extra validations
    if let Some(ref recipe) = construction_recipe {
        required_interior_tier = recipe.required_interior_tier;
        if let Some(claim_building) = ctx.db.building_claim_desc().building_id().find(&recipe.building_description_id) {
            if claim_building.claim_type == ClaimType::Source {
                if built_on_existing_claims {
                    return Err("Can't build on an existing claim".into());
                }

                claim_helper::can_place_claim_totem(ctx, coordinates.into(), &claim_building, &mut terrain_cache)?;
            } else if claim_building.claim_type == ClaimType::Extension {
                return Err("Extension totems are obsolete.".into());
            }
        }
    }

    // Resource-specific extra validations
    if let Some(recipe) = resource_placement_recipe {
        ProjectSiteState::validate_resource_placement(ctx, &recipe, &mut terrain_cache, coordinates)?;
        required_interior_tier = recipe.required_interior_tier;
    }

    if required_interior_tier == -1 && dimension.interior_instance_id != 0 {
        return Err("Can only be built in Overworld".into());
    }
    if required_interior_tier > 0 {
        if dimension.interior_instance_id == 0 {
            return Err(format!("Requires Tier {{0}} interior|~{}", required_interior_tier).into());
        }
        let instance = ctx.db.interior_instance_desc().id().find(&dimension.interior_instance_id).unwrap();
        if instance.tier < required_interior_tier {
            return Err(format!("Requires Tier {{0}} interior|~{}", required_interior_tier).into());
        }
    }

    let facing_direction = facing_direction as i32;

    // Empire buildings have a few extra rules
    if is_empire_building {
        if claim_helper::get_claim_under_footprint(ctx, &footprint) == 0 {
            return Err("This project site needs to be fully under an empire-aligned claim in order to be built".into());
        }
        let claim_entity_id = existing_claims_id.get(0).unwrap();
        if let Some(settlement) = EmpireSettlementState::from_claim(ctx, *claim_entity_id) {
            if settlement.empire_entity_id == 0 {
                return Err("This site needs to be fully under an empire-aligned claim in order to be built".into());
            }
            let player_data = unwrap_or_err!(
                ctx.db.empire_player_data_state().entity_id().find(&actor_id),
                "You need to be part of an empire to build this"
            );
            if settlement.empire_entity_id != player_data.empire_entity_id {
                return Err("You need to be part of the claim empire to build on this claim".into());
            }
        } else {
            return Err("This project site needs to be fully under an empire-aligned claim in order to be built".into());
        }

        if is_empire_foundry {
            let claim = ctx.db.claim_state().entity_id().find(claim_entity_id).unwrap();
            if ctx
                .db
                .empire_state()
                .capital_building_entity_id()
                .find(&claim.owner_building_entity_id)
                .is_none()
            {
                return Err("This building can only be built in the capital of an Empire".into());
            }

            validate_empire_build_foundry(ctx, actor_id, coordinates)?;
        }
    }

    let entity_id = game_state::create_entity(ctx);
    if is_claim_totem {
        let _ = ctx.db.auto_claim_state().try_insert(AutoClaimState {
            entity_id,
            owner_entity_id: actor_id,
        });
    }

    if is_bank || is_market {
        if existing_claims_id.len() != 1 {
            return Err("This building can only be built on a claim".into());
        }
        let claim_entity_id = existing_claims_id[0];
        if is_bank {
            if ctx.db.bank_state().claim_entity_id().filter(claim_entity_id).next().is_some() {
                return Err("Claims can only have one bank".into());
            }
            for project_site in ctx.db.project_site_state().owner_id().filter(claim_entity_id) {
                if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(project_site.construction_recipe_id) {
                    if let Some(building_desc) = ctx.db.building_desc().id().find(recipe.building_description_id) {
                        if building_desc.has_category(ctx, BuildingCategory::Bank) {
                            return Err("A bank is already under construction".into());
                        }
                    }
                }
            }
        }
        if is_market {
            if ctx
                .db
                .marketplace_state()
                .claim_entity_id()
                .filter(claim_entity_id)
                .next()
                .is_some()
            {
                return Err("Claims can only have one market".into());
            }
            for project_site in ctx.db.project_site_state().owner_id().filter(claim_entity_id) {
                if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(project_site.construction_recipe_id) {
                    if let Some(building_desc) = ctx.db.building_desc().id().find(recipe.building_description_id) {
                        if building_desc.has_category(ctx, BuildingCategory::TownMarket) {
                            return Err("A market is already under construction".into());
                        }
                    }
                }
            }
        }
    }

    //check built instantly
    let mut placed = false;
    if let Some(construction_recipe) = construction_recipe {
        if construction_recipe.instantly_built {
            placed = true;
            // Create building instantly
            let building_desc_id = construction_recipe.building_description_id; // Empire Foundry ID in the csv file. There should be a better way to do that. Using "Empire Foundry" is equally risky since we can change the name or case or spacing.
            create_building_unsafe(
                ctx,
                actor_id,
                Some(entity_id),
                coordinates,
                facing_direction,
                building_desc_id,
                Some(construction_recipe.id),
            )?;

            discover_instant_build(ctx, actor_id, construction_recipe.building_description_id, construction_recipe.id);
            consume_recipe_input(ctx, actor_id, Some(construction_recipe))?;

            player_action_helpers::post_reducer_update_cargo(ctx, actor_id);
        }
    }
    if !placed {
        let offset: crate::messages::util::OffsetCoordinatesSmallMessage = OffsetCoordinatesSmall::from(coordinates);

        let project_site = ProjectSiteState {
            entity_id,
            construction_recipe_id: request.construction_recipe_id,
            resource_placement_recipe_id: request.resource_placement_recipe_id,
            items: vec![],
            cargos: vec![],
            progress: 0,
            last_crit_outcome: 0,
            owner_id: project_owner_id,
            direction: facing_direction,
            last_hit_timestamp: ctx.timestamp,
        };

        if ctx.db.project_site_state().try_insert(project_site).is_err() {
            return Err("Failed to insert project site".into());
        }

        game_state::insert_location(ctx, entity_id, offset);
        create_project_site_footprint(ctx, entity_id, &footprint);
        footprint_helpers::clear_resources_under_footprint(ctx, &footprint, false);
    }

    PlayerActionState::success(
        ctx,
        actor_id,
        PlayerActionType::None,
        PlayerActionLayer::Base,
        0,
        None,
        None,
        game_state::unix_ms(ctx.timestamp),
    );

    Ok(())
}

fn validate_knowledge(ctx: &ReducerContext, actor_id: u64, required_knowledges: &Vec<i32>) -> Result<(), String> {
    for required_knowledge_id in required_knowledges {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *required_knowledge_id) {
            return Err("You don't have the knowledge required to build this".into());
        }
    }

    return Ok(());
}

fn discover_instant_build(ctx: &ReducerContext, actor_id: u64, building_id: i32, construction_recipe_id: i32) {
    let mut discovery = Discovery::new(actor_id);
    discovery.acquire_building(ctx, building_id);
    discovery.acquire_construction(ctx, construction_recipe_id);
    discovery.commit(ctx);
}

fn consume_recipe_input(ctx: &ReducerContext, actor_id: u64, recipe: Option<ConstructionRecipeDesc>) -> Result<(), String> {
    if let Some(construction_recipe) = recipe {
        let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
        let consumed_items = construction_recipe
            .consumed_item_stacks
            .iter()
            .map(|i| ItemStack::from_input(i))
            .collect();
        if !inventory.remove(&consumed_items) {
            return Err("Missing materials".into());
        }

        let consumed_cargo = construction_recipe
            .consumed_cargo_stacks
            .iter()
            .map(|i| ItemStack::from_input(i))
            .collect();
        if !inventory.remove(&consumed_cargo) {
            return Err("Missing materials".into());
        }

        //shards are consumed on global module

        ctx.db.inventory_state().entity_id().update(inventory);
    }
    Ok(())
}

pub fn refund_recipe_input(ctx: &ReducerContext, actor_id: u64, recipe: Option<i32>) {
    if recipe.is_some() {
        let construction_recipe = ctx.db.construction_recipe_desc().id().find(&recipe.unwrap()).unwrap();
        let mut inventory = unwrap_or_return!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
        let consumed_items = construction_recipe
            .consumed_item_stacks
            .iter()
            .map(|i| ItemStack::from_input(i))
            .collect();
        inventory.add_multiple_with_overflow(ctx, &consumed_items);

        let consumed_cargo = construction_recipe
            .consumed_cargo_stacks
            .iter()
            .map(|i| ItemStack::from_input(i))
            .collect();
        inventory.add_multiple_with_overflow(ctx, &consumed_cargo);

        ctx.db.inventory_state().entity_id().update(inventory);
    }
}

fn validate_rentability(ctx: &ReducerContext, dimension: u32, dimension_network: &DimensionNetworkState) -> Result<(), String> {
    let dimension_network_entity_id = dimension_network.entity_id;
    if dimension_network.claim_entity_id == 0 {
        return Err("You can only place rent terminals in claimed buildings".into());
    }
    // Validate that each interior in the network only has default spawned buildings
    for dimension_description in ctx
        .db
        .dimension_description_state()
        .dimension_network_entity_id()
        .filter(dimension_network_entity_id)
    {
        let interior_instance_id = dimension_description.interior_instance_id;

        let desc = unwrap_or_err!(
            ctx.db.interior_instance_desc().id().find(&interior_instance_id),
            "This dimension has no matching instance in static data"
        );

        if !desc.rentable {
            return Err("This building can't be rented".into());
        }

        // If there's any other player-made building or resource (or project site), we can't build.
        // Note: this could possibly be done and cached in a table, but it is not that expensive and only happens when
        // a player want to put a new construction site for a Rent Terminal in a dimension, so... not often
        let mut building_spawns: Vec<i32> = Vec::new();
        let mut resource_spawns: Vec<i32> = Vec::new();

        for spawn in ctx.db.interior_spawn_desc().interior_instance_id().filter(interior_instance_id) {
            match spawn.spawn_type {
                InteriorSpawnType::Building => building_spawns.push(spawn.building_id),
                InteriorSpawnType::Resource => {
                    resource_spawns.append(&mut ResourceClumpDesc::get_resource_ids(ctx, spawn.resource_clump_id))
                }
                InteriorSpawnType::InteriorSpawnRandomGroup => {
                    if let Some(group) = ctx
                        .db
                        .interior_spawn_random_group_desc()
                        .id()
                        .find(&spawn.interior_spawn_random_group_id)
                    {
                        for resource_clump_id in group.resource_clump_ids {
                            resource_spawns.append(&mut ResourceClumpDesc::get_resource_ids(ctx, resource_clump_id));
                        }
                    }
                }
                _ => (),
            }
        }
        building_spawns.sort();
        building_spawns.dedup();
        resource_spawns.sort();
        resource_spawns.dedup();

        let all_locations = LocationState::select_all_in_interior_dimension_iter(ctx, dimension);

        for l in all_locations {
            if ctx.db.project_site_state().entity_id().find(&l.entity_id).is_some() {
                return Err("You need to empty the building first before adding a rent terminal".into());
            }

            if let Some(building) = ctx.db.building_state().entity_id().find(&l.entity_id) {
                if !building_spawns.contains(&building.building_description_id) {
                    return Err("You need to empty the building first before adding a rent terminal".into());
                }
            } else if let Some(deposit) = ctx.db.resource_state().entity_id().find(&l.entity_id) {
                if !resource_spawns.contains(&deposit.resource_id) {
                    return Err("You need to empty the building first before adding a rent terminal".into());
                }
            }
        }
    }

    Ok(())
}
