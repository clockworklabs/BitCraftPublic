use std::cell::Cell;

use spacetimedb::{log, ReducerContext, Table, Timestamp};

use super::{
    footprint_helpers::{self, clear_resources_under_footprint, create_footprint, delete_footprint},
    interior_helpers::{create_building_interior, find_teleport_coordinates_for_interior_destruction},
};
use crate::{
    building_claim_desc, building_desc, claim_tech_desc,
    game::{
        autogen::_delete_entity::delete_entity,
        claim_helper,
        coordinates::*,
        dimensions,
        game_state::{self, game_state_filters},
        handlers::{
            claim::{claim_take_ownership, claim_tech_unlock_tech::claim_tech_unlock_timer},
            player::player_housing_change_entrance,
            server::{
                destroy_dimension_network::{destroy_dimension_network_timer, DestroyDimensionNetworkTimer},
                server_teleport_player::{teleport_player_timer, TeleportPlayerTimer},
            },
        },
        npc_empire::NPC_EMPIRE_DEFAULT_NAME,
    },
    inter_module::{self, send_inter_module_message},
    interior_network_desc,
    messages::{
        action_request::ServerTeleportReason,
        empire_shared::{
            empire_chunk_state, empire_node_state, empire_player_data_state, empire_settlement_state, empire_state, EmpireOwnerType,
        },
        game_util::ItemStack,
        inter_module::{DeleteEmpireMsg, GlobalDeleteEmpireBuildingMsg, MessageContentsV5, NpcPlaceWatchtowersMsg, NpcWatchtowerPlacement},
        static_data::{distant_visible_entity_desc, EntityType},
    },
    parameters_desc, unwrap_or_err, BuildingCategory, BuildingDesc, BuildingInteractionLevel, BuildingSpawnDesc, ClaimState, ClaimType,
    FootprintType, LightSourceState, TerraformProgressState,
};

use crate::messages::components::*;

pub fn create_building_unsafe(
    ctx: &ReducerContext,
    actor_id: u64,
    entity_id: Option<u64>,
    location: SmallHexTile,
    direction: i32,
    building_desc_id: i32,
    construction_recipe_id: Option<i32>,
) -> Result<(), String> {
    let building_desc = ctx.db.building_desc().id().find(&building_desc_id).unwrap();

    let entity_id = entity_id.unwrap_or(game_state::create_entity(ctx));

    game_state::insert_location(ctx, entity_id, location.into());

    let footprint = building_desc.get_footprint(&location, direction);
    let claim_entity_id = claim_helper::get_claim_under_footprint(ctx, &footprint);

    create_building_component(ctx, claim_entity_id, entity_id, direction, &building_desc, actor_id);

    create_building_footprint(ctx, entity_id, direction, &building_desc, &None);

    clear_resources_under_building(ctx, location.into(), &building_desc, direction);

    let neutral = match ctx.db.claim_state().entity_id().find(&claim_entity_id) {
        Some(c) => c.neutral,
        None => false,
    };

    create_building_claim(ctx, entity_id, neutral)?;

    if ctx.db.interior_network_desc().building_id().find(&building_desc.id).is_some() {
        create_building_interior(ctx, entity_id)?;
    }

    BuildingState::create_rental(ctx, entity_id, &building_desc, location.dimension)?;

    BuildingState::create_empire_building(ctx, entity_id, &building_desc, actor_id, location, construction_recipe_id);

    BuildingState::create_waystone(ctx, entity_id, claim_entity_id, &building_desc, location);
    BuildingState::create_bank(ctx, entity_id, claim_entity_id, &building_desc, location)?;
    BuildingState::create_marketplace(ctx, entity_id, claim_entity_id, &building_desc, location)?;

    create_distant_visibile_building(ctx, &building_desc, entity_id, location);

    create_building_spawns(ctx, entity_id);

    return Ok(());
}

pub fn create_building_component(
    ctx: &ReducerContext,
    owning_claim_entity_id: u64,
    building_entity_id: u64,
    direction: i32,
    building: &BuildingDesc,
    actor_id: u64,
) -> u64 {
    if building.build_permission == BuildingInteractionLevel::Empire {
        if let Some(empire_rank) = ctx.db.empire_player_data_state().entity_id().find(&actor_id) {
            if let Some(empire) = ctx.db.empire_state().entity_id().find(&empire_rank.empire_entity_id) {
                let nickname = format!("{}'s {}", empire.name, building.name);
                BuildingNicknameState::insert_shared(
                    ctx,
                    BuildingNicknameState {
                        entity_id: building_entity_id,
                        nickname,
                    },
                    crate::inter_module::InterModuleDestination::Global,
                );
            }
        }
    }

    let mut building_state = BuildingState {
        entity_id: building_entity_id,

        claim_entity_id: 0, // will claim via the BuildingState:claim() function. Streamlined code at the expanse of an extra update.
        direction_index: direction,
        building_description_id: building.id,
        constructed_by_player_entity_id: actor_id,
    };

    building_state.update_inventories(ctx, &building);

    //Insert building into the table (local or shared for waystones)
    if ctx.db.building_state().try_insert(building_state).is_err() {
        log::error!("Failed to insert building");
    }

    BuildingState::claim(ctx, building_entity_id, owning_claim_entity_id);

    if building.has_category(ctx, BuildingCategory::Waystone) {
        let building_state = ctx.db.building_state().entity_id().find(building_entity_id).unwrap();
        ctx.db.building_state().entity_id().delete(building_entity_id);
        BuildingState::insert_shared(ctx, building_state, crate::inter_module::InterModuleDestination::AllOtherRegions);
        let loc = ctx.db.location_state().entity_id().find(building_entity_id).unwrap();
        ctx.db.location_state().entity_id().delete(building_entity_id);
        LocationState::insert_shared(ctx, loc, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    // +Targetable
    // DAB Note: Targetable category should be part of the building desc
    let targetable_state = TargetableState::new(building_entity_id);
    ctx.db.targetable_state().try_insert(targetable_state).unwrap();

    ctx.db
        .attack_outcome_state()
        .try_insert(AttackOutcomeState::new(building_entity_id))
        .unwrap();

    ctx.db
        .combat_state()
        .try_insert(CombatState::new(building_entity_id, Vec::new()))
        .unwrap();

    ctx.db
        .health_state()
        .try_insert(HealthState {
            entity_id: building_entity_id,

            health: building.max_health as f32,
            last_health_decrease_timestamp: ctx.timestamp,
            died_timestamp: 0,
        })
        .unwrap();

    for i in 0..building.functions.len() {
        let function = &building.functions[i];

        //add terraform progress state if function terraforms
        if function.terraform {
            if ctx
                .db
                .terraform_progress_state()
                .try_insert(TerraformProgressState {
                    entity_id: building_entity_id,
                    final_height_target: i16::MAX,
                    next_height_target: i16::MAX,
                    progress: i32::MIN,
                })
                .is_err()
            {
                log::error!("Failed to insert terraform progress state");
            } else {
                log::info!("Inserted terraform progress state {}", building_entity_id);
            }
        }

        if function.trade_orders > 0 {
            let barter_stall_state = BarterStallState {
                entity_id: building_entity_id,
                market_mode_enabled: false,
            };

            if ctx.db.barter_stall_state().try_insert(barter_stall_state).is_err() {
                log::error!("Failed to insert barter stall state");
            }
        }
    }

    if building.light_radius > 0 {
        let _ = ctx.db.light_source_state().try_insert(LightSourceState {
            entity_id: building_entity_id,
            radius: building.light_radius as f32,
        });
    }

    building_entity_id
}

pub fn create_building_spawns(ctx: &ReducerContext, building_entity_id: u64) {
    // Spawns within spawns
    if DONT_CREATE_BUILDING_SPAWNS_COUNTER.get() <= 0 {
        if let Err(err) = BuildingSpawnDesc::spawn_all(ctx, building_entity_id) {
            log::error!("{}", err);
        }
    }
}

pub fn create_building_footprint(
    ctx: &ReducerContext,
    entity_id: u64,
    direction: i32,
    building: &BuildingDesc,
    override_type_fn: &Option<fn(&FootprintType) -> FootprintType>,
) {
    let location = ctx.db.location_state().entity_id().find(&entity_id).unwrap();
    let coordinates = location.coordinates();
    let footprint = building.get_footprint(&coordinates, direction);
    create_footprint(ctx, entity_id, &footprint, override_type_fn)
}

pub fn create_building_claim(ctx: &ReducerContext, building_entity_id: u64, neutral: bool) -> Result<u64, String> {
    let coordinates = game_state_filters::coordinates_any(ctx, building_entity_id);
    let mut existing_claim_id = 0;
    if let Some(claim) = claim_helper::get_claim_on_tile(ctx, coordinates) {
        existing_claim_id = claim.claim_id;
    }

    let mut building_state = match ctx.db.building_state().entity_id().find(&building_entity_id) {
        Some(b) => b,
        None => return Err("Building does not exist".into()),
    };
    let building_claim = ctx
        .db
        .building_claim_desc()
        .building_id()
        .find(&building_state.building_description_id);

    if let Some(claim_info) = building_claim {
        // Create new claim entity
        let claim_desc_entity_id = game_state::create_entity(ctx);
        if claim_info.claim_type == ClaimType::Source || claim_info.claim_type == ClaimType::Neutral {
            let neutral_claim_type = claim_info.claim_type == ClaimType::Neutral;
            let supplies = if neutral_claim_type {
                0
            } else {
                ctx.db.parameters_desc().version().find(&0).unwrap().starting_supplies
            };

            //set claim name
            let coord_large = coordinates.parent_large_tile().to_offset_coordinates();
            let mut name = format!("Claimed area (N: {}, E: {})", coord_large.z, coord_large.x);
            if neutral_claim_type {
                let building_desc_option = ctx.db.building_desc().id().find(&building_state.building_description_id);
                if let Some(building_desc) = building_desc_option {
                    name = format!("{} (N: {}, E: {})", building_desc.name, coord_large.z, coord_large.x);
                }
            }

            let num_tiles = match neutral {
                true => 0,
                false => ClaimState::num_tiles_in_radius(claim_info.radius),
            };
            let num_tile_neighbors = match neutral {
                true => 0,
                false => ClaimState::num_neighbors_in_radius(claim_info.radius) as u32,
            };

            // Note: We're making the claims global for now to allow the owner to have an UI button at any time.
            //       This will also be helpful if we add a Claim apply system and need to list them or if we add the claims button
            //       for every member with a way to filter the claims they're part of.
            //       Simply uncomment the code below and update game_client::send_initial_state_data to re-localize the claim descriptions

            let offset_coord = OffsetCoordinatesSmall::from(coordinates);
            // Add claim description component
            let claim_state: ClaimState = ClaimState {
                entity_id: claim_desc_entity_id,
                owner_player_entity_id: 0, // no owner by default until claimed
                owner_building_entity_id: building_entity_id,
                name,
                neutral: neutral || neutral_claim_type,
            };
            let claim_local_state = ClaimLocalState {
                entity_id: claim_desc_entity_id,
                supplies,
                num_tiles,
                num_tile_neighbors,
                location: Some(offset_coord),
                building_maintenance: 0.0,
                treasury: 0,
                xp_gained_since_last_coin_minting: 0,
                supplies_purchase_price: 1.0,
                supplies_purchase_threshold: 0,
                building_description_id: building_state.building_description_id,
            };
            ClaimState::insert_shared(ctx, claim_state, crate::inter_module::InterModuleDestination::Global);
            ctx.db.claim_local_state().insert(claim_local_state);

            // starts with tier 0 claim techs by default
            let starting_techs: Vec<i32> = ctx.db.claim_tech_desc().tier().filter(0).map(|ct| ct.id).collect();
            let claim_tech = ClaimTechState {
                entity_id: claim_desc_entity_id,
                learned: starting_techs,
                researching: 0,
                start_timestamp: Timestamp::UNIX_EPOCH,
                scheduled_id: None,
            };

            if ctx.db.claim_tech_state().try_insert(claim_tech).is_err() {
                return Err("Failed to insert claim tech".into());
            }

            if !claim_helper::claim_area_around_totem(ctx, claim_desc_entity_id, claim_info.radius, neutral | neutral_claim_type) {
                return Err("Too close to another claim area.".into());
            }

            // Attempt claiming
            if let Some(autoclaim) = ctx.db.auto_claim_state().entity_id().find(&building_entity_id) {
                let _ = claim_take_ownership::reduce(ctx, autoclaim.owner_entity_id, claim_desc_entity_id)?;
                ctx.db.auto_claim_state().entity_id().delete(&building_entity_id);
            }

            building_state.claim_entity_id = claim_desc_entity_id;
            ctx.db.building_state().entity_id().update(building_state);

            return Ok(claim_desc_entity_id);
        }
    }
    Ok(existing_claim_id)
}

pub fn delete_building(
    ctx: &ReducerContext,
    actor_id: u64, // for dropped inventory ownership
    entity_id: u64,
    interior_teleport_destination: Option<OffsetCoordinatesFloat>,
    ignore_portals: bool,
    drop_inventory_items: bool,
) {
    let building = ctx.db.building_state().entity_id().find(&entity_id).unwrap();
    let location = ctx.db.location_state().entity_id().find(&entity_id).unwrap().coordinates();
    let description = ctx.db.building_desc().id().find(&building.building_description_id).unwrap();

    let mut all_expelled_players = Vec::new();

    // Destroying a building with player housing will leave the dimension intact, so we need to expel the players manually
    for player_housing in ctx.db.player_housing_state().entrance_building_entity_id().filter(&entity_id) {
        let mut expelled_players = player_housing.expel_players_and_entities(ctx, ServerTeleportReason::PlayerHousingDeconstructed);
        all_expelled_players.append(&mut expelled_players);
    }

    LostItemsState::generate_lost_items_for_building(ctx, &building, &description);

    if let Some(portal) = ctx.db.portal_state().entity_id().find(&entity_id) {
        if !ignore_portals && portal.destination_dimension != dimensions::OVERWORLD {
            //Building has an interior. Teleport all players outside
            let dimension = ctx
                .db
                .dimension_description_state()
                .dimension_id()
                .find(&portal.destination_dimension)
                .unwrap();
            let dimension_network_id = dimension.dimension_network_entity_id;
            let dimensions: Vec<u32> = ctx
                .db
                .dimension_description_state()
                .dimension_network_entity_id()
                .filter(dimension_network_id)
                .map(|a| a.dimension_id)
                .collect();
            let teleport_oc_float = match interior_teleport_destination {
                Some(oc) => oc,
                None => find_teleport_coordinates_for_interior_destruction(ctx, entity_id),
            };
            let teleport_oc_small: OffsetCoordinatesSmall = teleport_oc_float.into();

            for mut player_state in ctx.db.player_state().iter() {
                let player_entity_id = player_state.entity_id;
                let coordinates_float = game_state_filters::coordinates_float(ctx, player_entity_id);

                if dimensions.contains(&coordinates_float.dimension) && !all_expelled_players.contains(&player_entity_id) {
                    ctx.db
                        .teleport_player_timer()
                        .try_insert(TeleportPlayerTimer {
                            scheduled_id: 0,
                            scheduled_at: ctx.timestamp.into(),
                            player_entity_id,
                            location: teleport_oc_float,
                            reason: ServerTeleportReason::InteriorDeconstructed,
                        })
                        .ok()
                        .unwrap();
                }

                let home_location = player_state.teleport_location.location;
                if dimensions.contains(&home_location.dimension) {
                    player_state.teleport_location.location = teleport_oc_small;

                    ctx.db.player_state().entity_id().update(player_state);
                }
            }

            ctx.db
                .destroy_dimension_network_timer()
                .try_insert(DestroyDimensionNetworkTimer {
                    scheduled_id: 0,
                    scheduled_at: ctx.timestamp.into(),
                    dimension_network_entity_id: dimension_network_id,
                    player_teleport_location: teleport_oc_float,
                })
                .ok()
                .unwrap();
        }
    }

    // Destroying an empire capital effectively destroys the empire
    if let Some(empire) = ctx.db.empire_state().capital_building_entity_id().find(&building.entity_id) {
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::DeleteEmpire(DeleteEmpireMsg {
                empire_entity_id: empire.entity_id,
            }),
            crate::inter_module::InterModuleDestination::Global,
        );
    }

    // Destroying a claim totem destroys the claim description and all the tiles
    if let Some(claim) = ctx.db.claim_state().owner_building_entity_id().find(&building.entity_id) {
        let claim_entity_id: u64 = claim.entity_id;

        // find any market place tied to this claim, and generate the lost items from it
        // this could be somewhat expensive if a massive claim gets destroyed, but this should happen very rarely if ever.
        if ctx
            .db
            .building_state()
            .claim_entity_id()
            .filter(claim_entity_id)
            .filter(|b| {
                ctx.db
                    .building_desc()
                    .id()
                    .find(b.building_description_id)
                    .unwrap()
                    .has_category(ctx, BuildingCategory::TownMarket)
            })
            .next()
            .is_some()
        {
            let location = game_state_filters::coordinates(ctx, building.entity_id);
            LostItemsState::generate_lost_items_from_market(ctx, claim_entity_id, location);
        }

        claim_helper::delete_all_claim_tiles(ctx, claim_entity_id);

        if let Some(claim_tech) = ctx.db.claim_tech_state().entity_id().find(&claim.entity_id) {
            if let Some(scheduled_id) = claim_tech.scheduled_id {
                ctx.db.claim_tech_unlock_timer().scheduled_id().delete(&scheduled_id);
            }
        }

        ClaimState::clear_notifications(ctx, claim.entity_id);
        ClaimState::delete_shared(ctx, claim, crate::inter_module::InterModuleDestination::Global);
        if let Some(lowercase) = ctx.db.claim_lowercase_name_state().entity_id().find(claim_entity_id) {
            //Technically this is owned by global module, but this saves us an IMM and should be safe enough
            ClaimLowercaseNameState::delete_shared(
                ctx,
                lowercase,
                crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
            );
        }

        delete_entity(ctx, claim_entity_id);
    }

    if drop_inventory_items {
        // Delete pocket entities, drop items and cargo for completed active crafts
        let mut items_to_drop: Vec<ItemStack> = Vec::new();

        // Drop items and cargo for inventory (includes completed passive crafts)
        for inventory in ctx.db.inventory_state().owner_entity_id().filter(entity_id) {
            for pocket in inventory.pockets {
                if let Some(contents) = pocket.contents {
                    items_to_drop.push(contents);
                }
            }
        }

        if items_to_drop.len() != 0 {
            DroppedInventoryState::update_from_items(ctx, actor_id, location.into(), items_to_drop, None);
        }
    }

    // Delete NPCs in this building
    for npc in ctx.db.npc_state().building_entity_id().filter(entity_id) {
        let npc_entity_id = npc.entity_id;
        npc.delete_trade_orders(ctx);
        ctx.db.npc_state().entity_id().delete(&npc_entity_id);
    }

    // Delete attached herds
    AttachedHerdsState::delete(ctx, entity_id);

    ctx.db.progressive_action_state().building_entity_id().delete(&entity_id);
    ctx.db.public_progressive_action_state().building_entity_id().delete(&entity_id);

    ctx.db.terraform_progress_state().entity_id().delete(&entity_id);

    ctx.db.passive_craft_state().building_entity_id().delete(&entity_id);

    delete_footprint(ctx, entity_id);

    // Clear all combat sessions involving the dead entity
    ThreatState::clear_all(ctx, entity_id);

    ctx.db.combat_state().entity_id().delete(&entity_id);
    ctx.db.targetable_state().entity_id().delete(&entity_id);
    ctx.db.attack_outcome_state().entity_id().delete(&entity_id);

    // delete light stamps
    if description.light_radius > 0 {
        ctx.db.light_source_state().entity_id().delete(&entity_id);
    }

    BuildingState::unclaim(ctx, entity_id, true);

    // Un-rent the matching interior
    if description.has_category(ctx, BuildingCategory::RentTerminal) {
        let mut dimension_network = DimensionNetworkState::get(ctx, location.dimension).unwrap();
        let dimension_network_description_entity_id = dimension_network.entity_id;
        dimension_network.rent_entity_id = 0;
        ctx.db.dimension_network_state().entity_id().update(dimension_network);
        ctx.db
            .rent_state()
            .dimension_network_id()
            .delete(&dimension_network_description_entity_id);
    }

    // delete inventories associated with this building
    let inventories = ctx
        .db
        .inventory_state()
        .owner_entity_id()
        .filter(entity_id)
        .map(|inv| inv.entity_id);
    for inv in inventories {
        ctx.db.inventory_state().entity_id().delete(&inv);
    }

    // delete empire stuff
    if ctx.db.empire_node_state().entity_id().find(&entity_id).is_some()
        || ctx.db.empire_settlement_state().building_entity_id().find(&entity_id).is_some()
    {
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg {
                player_entity_id: actor_id,
                building_entity_id: entity_id,
            }),
            crate::inter_module::InterModuleDestination::Global,
        );
    }

    if let Some(nickname) = ctx.db.building_nickname_state().entity_id().find(&entity_id) {
        BuildingNicknameState::delete_shared(ctx, nickname, crate::inter_module::InterModuleDestination::Global);
    }

    // finally delete building entity and remaining components
    ctx.db.health_state().entity_id().delete(&entity_id);
    if description.has_category(ctx, BuildingCategory::Waystone) {
        let loc = ctx
            .db
            .location_state()
            .entity_id()
            .find(entity_id)
            .expect("LocationState not found");
        LocationState::delete_shared(ctx, loc, crate::inter_module::InterModuleDestination::AllOtherRegions);
        BuildingState::delete_shared(ctx, building, crate::inter_module::InterModuleDestination::AllOtherRegions);
    } else {
        ctx.db.location_state().entity_id().delete(&entity_id);
        ctx.db.building_state().entity_id().delete(&entity_id);
    }
    ctx.db.barter_stall_state().entity_id().delete(&entity_id);
    ctx.db.player_note_state().entity_id().delete(&entity_id);
    ctx.db.distant_visible_entity().entity_id().delete(&entity_id);

    ctx.db.waystone_state().building_entity_id().delete(entity_id);
    ctx.db.bank_state().building_entity_id().delete(entity_id);
    ctx.db.marketplace_state().building_entity_id().delete(entity_id);
}

pub fn clear_resources_under_building(ctx: &ReducerContext, coordinates: SmallHexTile, building: &BuildingDesc, facing_direction: i32) {
    let footprint = building.get_footprint(&coordinates, facing_direction);
    clear_resources_under_footprint(ctx, &footprint, false)
}

pub fn create_distant_visibile_building(ctx: &ReducerContext, building_desc: &BuildingDesc, entity_id: u64, coordinates: SmallHexTile) {
    if ctx
        .db
        .distant_visible_entity_desc()
        .description_id()
        .filter(building_desc.id)
        .any(|x| x.entity_type == EntityType::Building)
    {
        ctx.db.distant_visible_entity().insert(DistantVisibleEntity {
            entity_id: entity_id,
            chunk_index: coordinates.chunk_coordinates().chunk_index(),
        });
    }
}

pub fn move_building_unsafe(
    ctx: &ReducerContext,
    building_entity_id: u64,
    coordinates: OffsetCoordinatesSmall,
    facing_direction: i32,
) -> Result<(), String> {
    let mut building_state = unwrap_or_err!(ctx.db.building_state().entity_id().find(&building_entity_id), "Invalid building");
    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building_state.building_description_id),
        "Invalid building description"
    );
    let previous_coordinates = game_state_filters::coordinates_any(ctx, building_entity_id);
    let prev_direction = building_state.direction_index;

    // Delete previous footprints
    footprint_helpers::delete_footprint(ctx, building_entity_id);

    if building_desc.has_category(ctx, BuildingCategory::TradingPost) {
        if let Some(npc_state) = ctx
            .db
            .npc_state()
            .building_entity_id()
            .filter(&building_entity_id)
            .into_iter()
            .next()
        {
            if let Some(mut mobile_entity_state) = ctx.db.mobile_entity_state().entity_id().find(&npc_state.entity_id) {
                mobile_entity_state.set_location(coordinates.into());
                mobile_entity_state.set_destination(coordinates.into());

                ctx.db.mobile_entity_state().entity_id().update(mobile_entity_state);
            }
        }
    }

    let offset_coordinates = OffsetCoordinatesSmall::from(coordinates);
    building_state.direction_index = facing_direction;
    ctx.db.building_state().entity_id().update(building_state);

    let location = LocationState::new(building_entity_id, offset_coordinates);
    let location_hex = location.coordinates();
    if building_desc.has_category(ctx, BuildingCategory::Waystone) {
        LocationState::update_shared(ctx, location, crate::inter_module::InterModuleDestination::AllOtherRegions);
    } else {
        ctx.db.location_state().entity_id().update(location);
    }
    // Update new footprints
    create_building_footprint(ctx, building_entity_id, facing_direction, &building_desc, &None);

    clear_resources_under_building(ctx, coordinates.into(), &building_desc, facing_direction);

    //Move interior portals that exit to this building
    if ctx.db.interior_network_desc().building_id().find(&building_desc.id).is_some() {
        let portals = ctx.db.portal_state().target_building_entity_id().filter(building_entity_id);
        let prev_offset_coordinates = OffsetCoordinatesSmall::from(previous_coordinates);
        for mut portal in portals {
            let prev_coord = SmallHexTile::from(OffsetCoordinatesSmall {
                x: portal.destination_x,
                z: portal.destination_z,
                dimension: prev_offset_coordinates.dimension,
            });
            let prev_portal_origin = prev_coord.rotate_around(&previous_coordinates, -prev_direction / 2);
            let new_portal_origin = prev_portal_origin - previous_coordinates + location_hex;
            let new_portal_rotated = new_portal_origin.rotate_around(&location_hex, facing_direction / 2);
            let oc = OffsetCoordinatesSmall::from(new_portal_rotated);
            portal.destination_x = oc.x;
            portal.destination_z = oc.z;
            ctx.db.portal_state().entity_id().update(portal);
        }
    }

    //Move housing portals
    for housing in ctx
        .db
        .player_housing_state()
        .entrance_building_entity_id()
        .filter(building_entity_id)
    {
        if let Some(portal) = ctx.db.portal_state().entity_id().find(housing.exit_portal_entity_id) {
            let _ = player_housing_change_entrance::update_portal_position(ctx, portal);
        }
    }

    //Rotate NPCs
    if prev_direction != facing_direction {
        for mut npc in ctx.db.npc_state().building_entity_id().filter(building_entity_id) {
            npc.direction += facing_direction - prev_direction;
            ctx.db.npc_state().entity_id().update(npc);
        }
    }

    Ok(())
}

//Having an active instance of this struct will prevent claim_area_around_totem from running game_state_filters::claims_in_radius
pub struct DontCreateBuildingSpawnsSpan {
    initialized: bool,
}

impl DontCreateBuildingSpawnsSpan {
    pub fn start() -> Self {
        DONT_CREATE_BUILDING_SPAWNS_COUNTER.replace(DONT_CREATE_BUILDING_SPAWNS_COUNTER.get() + 1);
        Self { initialized: true }
    }

    pub fn end(self) {
        // just drop self
    }
}

impl std::ops::Drop for DontCreateBuildingSpawnsSpan {
    fn drop(&mut self) {
        if self.initialized {
            DONT_CREATE_BUILDING_SPAWNS_COUNTER.replace(DONT_CREATE_BUILDING_SPAWNS_COUNTER.get() - 1);
            self.initialized = false;
        }
    }
}

thread_local! {
    static DONT_CREATE_BUILDING_SPAWNS_COUNTER: Cell<i32> = Cell::new(0);
}

/// 5x5 grid around the watchtower center chunk (radius=2 means -2..=2 in both axes)
const WATCHTOWER_CHUNK_RADIUS: i32 = 2;

/// If the placed building is a watchtower, sets up NPC empire state by computing a 5x5 chunk grid
/// around the position, filtering out ocean and already-claimed chunks, and sending an inter-module
/// message to global for EmpireNodeState + EmpireChunkState creation.
pub fn setup_npc_watchtower_state(
    ctx: &ReducerContext,
    entity_id: u64,
    building_desc: &BuildingDesc,
    location: SmallHexTile,
    energy: i32,
    upkeep: i32,
) {
    if !building_desc.has_category(ctx, BuildingCategory::Watchtower) {
        return;
    }

    let center_chunk = location.chunk_coordinates();

    let mut chunk_indexes = Vec::new();
    for dz in -WATCHTOWER_CHUNK_RADIUS..=WATCHTOWER_CHUNK_RADIUS {
        for dx in -WATCHTOWER_CHUNK_RADIUS..=WATCHTOWER_CHUNK_RADIUS {
            let cx = center_chunk.x + dx;
            let cz = center_chunk.z + dz;

            if cx < 0 || cz < 0 {
                continue;
            }

            let chunk = ChunkCoordinates {
                x: cx,
                z: cz,
                dimension: center_chunk.dimension,
            };
            let idx = chunk.chunk_index();

            if ctx.db.terrain_chunk_state().chunk_index().find(&idx).is_none() {
                continue;
            }

            if ctx.db.empire_chunk_state().chunk_index().find(&idx).is_some() {
                continue;
            }

            chunk_indexes.push(idx);
        }
    }

    if chunk_indexes.is_empty() {
        log::warn!(
            "setup_npc_watchtower_state: no unclaimed chunks available for watchtower {}",
            entity_id
        );
        return;
    }

    let npc_empire_name = ctx
        .db
        .empire_state()
        .iter()
        .find(|e| e.owner_type == EmpireOwnerType::Npc)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| NPC_EMPIRE_DEFAULT_NAME.to_string());

    BuildingNicknameState::insert_shared(
        ctx,
        BuildingNicknameState {
            entity_id,
            nickname: format!("{}'s {}", npc_empire_name, building_desc.name),
        },
        inter_module::InterModuleDestination::Global,
    );

    log::info!(
        "setup_npc_watchtower_state: watchtower {} claiming {} chunks",
        entity_id,
        chunk_indexes.len()
    );

    send_inter_module_message(
        ctx,
        MessageContentsV5::NpcPlaceWatchtowers(NpcPlaceWatchtowersMsg {
            watchtowers: vec![NpcWatchtowerPlacement {
                building_entity_id: entity_id,
                location: location.into(),
                chunk_indexes,
            }],
            energy,
            upkeep,
        }),
        inter_module::InterModuleDestination::Global,
    );
}
