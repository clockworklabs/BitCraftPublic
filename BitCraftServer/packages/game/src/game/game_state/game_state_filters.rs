use hex_direction::HexDirection;
use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::region_coordinates::RegionCoordinates;
use crate::game::{reducer_helpers, PLAYER_MIN_SWIM_DEPTH};
use crate::messages::generic::world_region_state;
use crate::messages::static_data::BuildingCategory::Waystone;
use crate::messages::util::MovementSpeed;
use crate::utils::from_ctx::FromCtx;
use crate::{
    game::{
        claim_helper, coordinates::*, dimensions, entities::building_state::BuildingState, permission_helper,
        reducer_helpers::deployable_helpers, terrain_chunk::TerrainChunkCache,
    },
    messages::{components::*, static_data::*},
    unwrap_or_err,
};

pub fn coordinates(ctx: &ReducerContext, entity_id: u64) -> SmallHexTile {
    if let Some(location) = ctx.db.location_state().entity_id().find(&entity_id) {
        return location.coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn offset_coordinates(ctx: &ReducerContext, entity_id: u64) -> OffsetCoordinatesSmall {
    if let Some(location) = ctx.db.location_state().entity_id().find(&entity_id) {
        return location.offset_coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_float(ctx: &ReducerContext, entity_id: u64) -> FloatHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return location.coordinates_float();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn offset_coordinates_float(ctx: &ReducerContext, entity_id: u64) -> OffsetCoordinatesFloat {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return location.offset_coordinates_float();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_any(ctx: &ReducerContext, entity_id: u64) -> SmallHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return location.coordinates();
    }
    if let Some(location) = ctx.db.location_state().entity_id().find(&entity_id) {
        return location.coordinates();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn coordinates_try_get(ctx: &ReducerContext, entity_id: u64) -> Result<SmallHexTile, String> {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return Ok(location.coordinates());
    }
    if let Some(location) = ctx.db.location_state().entity_id().find(&entity_id) {
        return Ok(location.coordinates());
    }
    Err("Invalid entity location".into())
}

pub fn coordinates_any_float(ctx: &ReducerContext, entity_id: u64) -> FloatHexTile {
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return location.coordinates_float();
    }
    if let Some(location) = ctx.db.location_state().entity_id().find(&entity_id) {
        return location.coordinates().into();
    }
    panic!("No location for entity id {}", entity_id);
}

pub fn building_at_coordinates(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<BuildingState> {
    let id = building_id_at_coordinates(ctx, coordinates);
    match id {
        Some(id) => ctx.db.building_state().entity_id().find(&id),
        None => None,
    }
}

pub fn building_id_at_coordinates(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<u64> {
    for f in FootprintTileState::get_at_location(ctx, coordinates) {
        if f.footprint_type == FootprintType::Hitbox || f.footprint_type == FootprintType::Walkable {
            return Some(f.owner_entity_id);
        }
    }
    None
}

pub fn project_site_at_coordinates(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<ProjectSiteState> {
    for f in FootprintTileState::get_at_location(ctx, coordinates) {
        if f.footprint_type == FootprintType::Hitbox || f.footprint_type == FootprintType::Walkable {
            if let Some(ps) = ctx.db.project_site_state().entity_id().find(f.owner_entity_id) {
                return Some(ps);
            }
        }
    }
    None
}

pub fn deployables_at_coordinates<'a>(ctx: &'a ReducerContext, coordinates: SmallHexTile) -> impl Iterator<Item = DeployableState> + 'a {
    MobileEntityState::select_all(ctx, coordinates).filter_map(|x| ctx.db.deployable_state().entity_id().find(x.entity_id))
}

pub fn paving_at_coordinates(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<PavedTileState> {
    PavedTileState::get_at_location(ctx, coordinates)
}

pub fn has_hitbox_footprint(ctx: &ReducerContext, coordinates: SmallHexTile) -> bool {
    for footprint in FootprintTileState::get_at_location(ctx, &coordinates) {
        if footprint.footprint_type == FootprintType::Hitbox {
            return true;
        }
    }
    return false;
}

pub fn get_hitbox_footprint(ctx: &ReducerContext, coordinates: SmallHexTile) -> Option<FootprintTileState> {
    for footprint in FootprintTileState::get_at_location(ctx, &coordinates) {
        if footprint.footprint_type == FootprintType::Hitbox {
            return Some(footprint);
        }
    }
    return None;
}

pub fn is_flat_corner(ctx: &ReducerContext, terrain_cache: &mut TerrainChunkCache, coordinates: SmallHexTile) -> bool {
    let elevations: [i16; 3] = coordinates
        .get_terrain_coordinates()
        .map(|c| terrain_cache.get_terrain_cell(ctx, &c).unwrap_or_default().elevation);
    return elevations[0] == elevations[1] && elevations[1] == elevations[2];
}

pub fn is_submerged(ctx: &ReducerContext, terrain_cache: &mut TerrainChunkCache, coordinates: SmallHexTile) -> bool {
    if !coordinates.is_corner() {
        return terrain_cache
            .get_terrain_cell(ctx, &coordinates.parent_large_tile())
            .unwrap_or_default()
            .is_submerged();
    }

    return coordinates
        .get_terrain_coordinates()
        .iter()
        .any(|c| terrain_cache.get_terrain_cell(ctx, &c).unwrap_or_default().is_submerged());
}

pub fn enemies_in_radius<'a>(
    ctx: &'a ReducerContext,
    coord: SmallHexTile,
    radius: i32,
) -> impl Iterator<Item = (EnemyState, SmallHexTile)> + 'a {
    chunk_indexes_in_radius(coord, radius)
        .flat_map(|chunk_index| ctx.db.mobile_entity_state().chunk_index().filter(chunk_index))
        .filter(move |mes| mes.coordinates().distance_to(coord) <= radius)
        .filter_map(|mes| match ctx.db.enemy_state().entity_id().find(&mes.entity_id) {
            Some(enemy) => Some((enemy, mes.coordinates())),
            None => None,
        })
}

pub fn chunk_indexes_in_radius(coord: SmallHexTile, radius: i32) -> impl Iterator<Item = u64> {
    let chunk = coord.chunk_coordinates();
    let mut min_x = chunk.x;
    let mut min_z = chunk.z;
    let mut max_x = chunk.x;
    let mut max_z = chunk.z;

    let mut direction = HexDirection::NE;
    for _i in 0..6 {
        let chunk = coord.neighbor_n(direction, radius).chunk_coordinates();
        min_x = min_x.min(chunk.x);
        min_z = min_z.min(chunk.z);
        max_x = max_x.max(chunk.x);
        max_z = max_z.max(chunk.z);
        direction = HexDirection::previous_flat(direction);
    }

    let dimension = coord.dimension;
    (min_x..=max_x).flat_map(move |x| (min_z..=max_z).map(move |z| ChunkCoordinates { x, z, dimension }.chunk_index()))
}

pub fn any_claims_in_radius(ctx: &ReducerContext, coord: SmallHexTile, radius: i32) -> bool {
    return claims_in_radius_iter(ctx, coord, radius).next().is_some();
}

pub fn any_claim_totems_in_radius(ctx: &ReducerContext, coord: SmallHexTile, radius: i32) -> bool {
    BuildingDesc::claim_totems(ctx)
        .iter()
        .flat_map(|id| {
            ctx.db.building_state().building_description_id().filter(id).filter(|b| {
                if let Some(loc) = ctx.db.location_state().entity_id().find(&b.entity_id) {
                    loc.coordinates().distance_to(coord) <= radius
                } else {
                    false
                }
            })
        })
        .next()
        .is_some()
}

pub fn claims_in_radius_iter<'a>(ctx: &'a ReducerContext, coord: SmallHexTile, radius: i32) -> impl Iterator<Item = ClaimTileState> + 'a {
    if coord.dimension != dimensions::OVERWORLD {
        panic!("claims_in_radius should only be called in overworld");
    }

    SmallHexTile::coordinates_in_radius_with_center_iter(coord, radius).filter_map(|t| ClaimTileState::get_at_location(ctx, &t))
}

pub fn claims_in_radius_except(ctx: &ReducerContext, coord: SmallHexTile, radius: i32, entities_to_ignore: &Vec<u64>) -> Vec<u64> {
    let mut tiles = SmallHexTile::coordinates_in_radius(coord, radius);
    tiles.push(coord);
    let mut claims: Vec<u64> = Vec::new();

    for t in tiles {
        if let Some(claim) = claim_helper::get_claim_on_tile(ctx, t.clone()) {
            if !claims.contains(&claim.entity_id) && !entities_to_ignore.contains(&claim.entity_id) {
                claims.push(claim.entity_id);
            }
        }
    }
    claims
}

pub fn buildings_in_radius(ctx: &ReducerContext, coord: SmallHexTile, radius: i32) -> Vec<BuildingState> {
    let buildings = ctx
        .db
        .building_state()
        .iter()
        .filter(|e| {
            if let Some(loc) = ctx.db.location_state().entity_id().find(&e.entity_id) {
                loc.coordinates().distance_to(coord) <= radius
            } else {
                false
            }
        })
        .collect();
    buildings
}

pub fn project_sites_in_radius(ctx: &ReducerContext, coord: SmallHexTile, radius: i32) -> Vec<ProjectSiteState> {
    let sites = ctx
        .db
        .project_site_state()
        .iter()
        .filter(|e| {
            if let Some(loc) = ctx.db.location_state().entity_id().find(&e.entity_id) {
                loc.coordinates().distance_to(coord) <= radius
            } else {
                false
            }
        })
        .collect();
    sites
}

pub fn get_location_for_entity(ctx: &ReducerContext, entity_id: u64) -> Option<SmallHexTile> {
    if let Some(loc) = ctx.db.location_state().entity_id().find(&entity_id) {
        return Some(loc.coordinates());
    }
    if let Some(loc) = ctx.db.mobile_entity_state().entity_id().find(&entity_id) {
        return Some(loc.coordinates());
    }
    // Progressive actions locations are their building's
    if let Some(action) = ctx.db.progressive_action_state().entity_id().find(&entity_id) {
        if let Some(loc) = ctx.db.location_state().entity_id().find(&action.building_entity_id) {
            return Some(loc.coordinates());
        }
    }
    // Inventory locations are their owner's
    if let Some(inventory) = ctx.db.inventory_state().entity_id().find(&entity_id) {
        if let Some(loc) = ctx.db.mobile_entity_state().entity_id().find(&inventory.owner_entity_id) {
            // Player or Deployable Inventory
            return Some(loc.coordinates());
        }
        if let Some(loc) = ctx.db.location_state().entity_id().find(&inventory.owner_entity_id) {
            // Building Inventory
            return Some(loc.coordinates());
        }
    }
    // This is a global entity with no location
    None
}

pub fn untarget(ctx: &ReducerContext, entity_id: u64) {
    // clear anyone targetting the entity
    let attackers: Vec<u64> = ctx
        .db
        .target_state()
        .target_entity_id()
        .filter(entity_id)
        .map(|t| t.entity_id)
        .collect();
    for e in attackers {
        ctx.db.target_state().entity_id().delete(&e);
    }
}

pub fn award_experience_on_damage(ctx: &ReducerContext, damage: f32, entity_id: u64, attacker_entity_id: Option<u64>) {
    // Enemy damage
    if let Some(attacker_entity_id) = attacker_entity_id {
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(&entity_id) {
            let enemy_type = enemy.enemy_type as i32;
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

            // Grant experience to attacker
            let experience_per_damage_dealt = enemy_desc.experience_per_damage_dealt.get(0);
            if let Some(experience_per_damage_dealt) = experience_per_damage_dealt {
                ExperienceState::add_experience(
                    ctx,
                    attacker_entity_id,
                    experience_per_damage_dealt.skill_id,
                    f32::ceil(experience_per_damage_dealt.quantity * damage) as i32,
                );
            }
        }
    }
}

pub fn remove_entity_inventories(ctx: &ReducerContext, owner_entity_id: u64) {
    let inventories: Vec<u64> = ctx
        .db
        .inventory_state()
        .owner_entity_id()
        .filter(owner_entity_id)
        .map(|a| a.entity_id)
        .collect();
    for inventory in inventories {
        ctx.db.inventory_state().entity_id().delete(&inventory);
    }
}

pub fn inventory_index_for_function(ctx: &ReducerContext, building_id: i32, function_type: i32) -> Option<i32> {
    let mut inventory_index = -1;
    let mut ret_val = None;
    let building = ctx.db.building_desc().id().find(&building_id).unwrap();

    for function in building.functions {
        let has_inventory = function.cargo_slots != 0 || function.storage_slots != 0;
        if has_inventory {
            inventory_index = inventory_index + 1;
            if function.function_type == function_type {
                ret_val = Some(inventory_index);
                break;
            }
        }
    }
    ret_val
}

pub fn is_interior_tile_walkable(ctx: &ReducerContext, coord: SmallHexTile) -> bool {
    if coord.dimension != dimensions::OVERWORLD {
        let dimension_description = ctx.db.dimension_description_state().dimension_id().find(&coord.dimension).unwrap();
        let interior = ctx
            .db
            .interior_instance_desc()
            .id()
            .find(&dimension_description.interior_instance_id)
            .unwrap();
        let shape = ctx.db.interior_shape_desc().id().find(&interior.interior_shape_id).unwrap();
        let offset_coord = OffsetCoordinatesSmall::from(coord);
        return shape.footprint.iter().any(|f| {
            f.x - shape.min_x == offset_coord.x && f.z - shape.min_z == offset_coord.z && f.footprint_type != FootprintType::Hitbox
        });
    }
    true // although this is NOT inside an interior
}

pub fn teleport_home(ctx: &ReducerContext, actor_id: u64, from_death: bool) -> Result<(), String> {
    let player = ctx.db.player_state().entity_id().find(&actor_id).unwrap();
    let teleport_location = player.teleport_location;

    let teleport_location_float = OffsetCoordinatesFloat::from(teleport_location.location);

    // Innerlight buff
    let mut active_buff_state = unwrap_or_err!(ctx.db.active_buff_state().entity_id().find(&actor_id), "Player has no buff state");
    let innerlight_buff_duration = ctx.db.parameters_desc_v2().version().find(&0).unwrap().respawn_aggro_immunity;
    active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);

    let teleportation_energy_state = TeleportationEnergyState::get(ctx, actor_id);
    let energy_cost = teleportation_energy_state.teleport_home_cost(ctx, from_death);
    teleport_to(ctx, actor_id, teleport_location_float, true, energy_cost)?;

    Ok(())
}

pub fn teleport_waystone(
    ctx: &ReducerContext,
    actor_id: u64,
    building_entity_id_from: u64,
    building_entity_id_to: u64,
    dry_run: bool,
) -> Result<(), String> {
    let building_state_to = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id_to),
        "Building does not exist."
    );

    if building_state_to.claim_entity_id == 0 {
        return Err("Destination waystone is not claimed".into());
    }

    let building_to = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building_state_to.building_description_id),
        "Building does not exist."
    );

    // verify building to is charter stone
    if !building_to.has_category(ctx, Waystone) {
        return Err("Building is not a charter stone.".into());
    }

    let building_state_from = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id_from),
        "Building does not exist."
    );

    if building_state_from.claim_entity_id == 0 {
        return Err("This waystone is not claimed".into());
    }

    let building_from = unwrap_or_err!(
        ctx.db.building_desc().id().find(&building_state_from.building_description_id),
        "Building does not exist."
    );

    // verify building from is charter stone
    if !building_from.has_category(ctx, Waystone) {
        return Err("Building is not a charter stone.".into());
    }

    // verify from charter stone is nearby
    let location_state_from = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(&building_entity_id_from),
        "Charter stone does not exist."
    );
    let coords_from = location_state_from.coordinates();

    let player_location = ctx.db.mobile_entity_state().entity_id().find(&actor_id).unwrap();

    if player_location.coordinates().distance_to(coords_from) > 7 {
        let player_claim_tile = claim_helper::get_claim_on_tile(ctx, player_location.coordinates());
        if player_claim_tile.is_none() || player_claim_tile.unwrap().claim_id != building_state_from.claim_entity_id {
            return Err("Too far away".into());
        }
    }

    // verify charter stone to location exists
    let location_state_to = unwrap_or_err!(
        ctx.db.location_state().entity_id().find(&building_entity_id_to),
        "Charter stone does not exist."
    );

    // find adjacent coordinate
    let teleport_to_adjacent = SmallHexTile::coordinates_in_radius(location_state_to.coordinates(), 1);
    let coords_to = teleport_to_adjacent[0];

    let location = coords_to.to_offset_coordinates();
    let teleport_location = TeleportLocation {
        location,
        location_type: TeleportLocationType::Waystone,
    };

    let teleport_location_float = OffsetCoordinatesFloat::from(teleport_location.location);

    // Innerlight buff
    let mut active_buff_state = unwrap_or_err!(ctx.db.active_buff_state().entity_id().find(&actor_id), "Player has no buff state");
    let innerlight_buff_duration = ctx.db.parameters_desc_v2().version().find(&0).unwrap().respawn_aggro_immunity;
    active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);

    // verify teleportation energy
    let teleportation_energy_state = TeleportationEnergyState::get(ctx, actor_id);
    let energy_cost = teleportation_energy_state.teleport_cost(ctx, coords_to.into());
    if energy_cost.is_none() {
        return Err("Invalid teleportation points".into());
    }

    let energy_cost = energy_cost.unwrap();
    if teleportation_energy_state.energy < energy_cost {
        return Err("You don't have enough energy to teleport there".into());
    }

    if !dry_run {
        teleportation_energy_state.update(ctx);

        teleport_to(ctx, actor_id, teleport_location_float, true, energy_cost)?;
    }

    Ok(())
}

pub fn teleport_to(
    ctx: &ReducerContext,
    actor_id: u64,
    teleport_location: OffsetCoordinatesFloat,
    dismount_deployable: bool,
    teleport_energy_cost: f32,
) -> Result<(), String> {
    let previous_location = ctx
        .db
        .mobile_entity_state()
        .entity_id()
        .find(&actor_id)
        .unwrap()
        .coordinates_float();

    // Player Housing
    PlayerHousingState::update_is_empty_flag(ctx, previous_location.dimension);

    // end all combat sessions involving this player
    ThreatState::clear_all(ctx, actor_id);

    // clear anyone targetting the dead player
    untarget(ctx, actor_id);

    // Can only change region if going in the overworld. Checking the destination chunk coordinates is meaningless in interiors
    // as interiors always have chunk coordinates 0,0
    if teleport_location.dimension == dimensions::OVERWORLD {
        let region = ctx.db.world_region_state().iter().next().unwrap();
        let dst_module = RegionCoordinates::from_ctx(ctx, FloatHexTile::from(teleport_location)).to_region_index(region.region_count_sqrt);
        let is_different_module = dst_module != region.region_index;

        if is_different_module {
            if !dismount_deployable {
                panic!("Can't do inter-module teleport without dismounting deployables");
            }
            deployable_helpers::dismount_deployable(ctx, actor_id, false);
            crate::inter_module::transfer_player::send_message(ctx, actor_id, teleport_location.into(), false, teleport_energy_cost);
            return Ok(());
        } else if teleport_energy_cost > 0.0 {
            // consume energy for local teleport. Off region teleport will consume energy when arriving at destination module
            let mut teleportation_energy_state = TeleportationEnergyState::get(ctx, actor_id);
            teleportation_energy_state.expend_energy(teleport_energy_cost, true); // allow negative energy ; waystone teleport already checked requisites
            teleportation_energy_state.update(ctx);
        }
    }

    let mut moved_player = false;
    if dismount_deployable {
        // remove deployable when teleporting (from death or teleport command)
        deployable_helpers::dismount_deployable(ctx, actor_id, false);
    } else {
        if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&actor_id) {
            // teleport vehicle and passengers
            reducer_helpers::deployable_helpers::move_deployable(
                ctx,
                mounting.deployable_entity_id,
                previous_location.into(),
                teleport_location,
                ctx.timestamp.to_micros_since_unix_epoch() as u64,
                0.0,
            )?;

            // Force the teleport on vehicle and all passengers
            let mut mobile_entity = MobileEntityState::for_location(mounting.deployable_entity_id, teleport_location, ctx.timestamp);
            let new_chunk_index = mobile_entity.chunk_index;
            ctx.db.mobile_entity_state().entity_id().update(mobile_entity.clone());
            PlayerActionState::update_chunk_index_on_all_layers(ctx, actor_id, new_chunk_index);

            for passenger in ctx.db.mounting_state().deployable_entity_id().filter(mounting.deployable_entity_id) {
                mobile_entity.entity_id = passenger.entity_id;
                ctx.db.mobile_entity_state().entity_id().update(mobile_entity.clone());
                PlayerActionState::update_chunk_index_on_all_layers(ctx, passenger.entity_id, new_chunk_index);
            }
            moved_player = true;
        }
    }

    if !moved_player {
        // update movement data, discoveries, and move player
        PlayerState::move_player_and_explore(ctx, actor_id, &previous_location, &teleport_location.into(), 0.0, false, None)?;
        // force the teleport
        let mobile_entity = MobileEntityState::for_location(actor_id, teleport_location, ctx.timestamp);
        let new_chunk_index = mobile_entity.chunk_index;
        ctx.db.mobile_entity_state().entity_id().update(mobile_entity);
        PlayerActionState::update_chunk_index_on_all_layers(ctx, actor_id, new_chunk_index);
    }

    Ok(())
}

pub fn validate_barter_permissions(ctx: &ReducerContext, actor_id: u64, building: &BuildingState, dimension: u32) -> Result<(), String> {
    // This takes care of rental permissions and default non-members

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Inventory) {
        return Err("You don't have permission".into());
    }

    if !permission_helper::can_interact_with_building(ctx, &building, actor_id, ClaimPermission::Inventory) {
        return Err("You don't have permission".into());
    }

    let dimension = ctx.db.dimension_description_state().dimension_id().find(&dimension);
    // Check for co-owner role if not part of a rental
    if dimension.is_none()
        || ctx
            .db
            .rent_state()
            .dimension_network_id()
            .find(&dimension.unwrap().dimension_network_entity_id)
            .is_none()
    {
        let claim = unwrap_or_err!(
            ctx.db.claim_state().entity_id().find(&building.claim_entity_id),
            "Building is not claimed"
        );
        if !claim.has_co_owner_permissions(ctx, actor_id) {
            return Err("You don't have permission".into());
        }
    }
    // Co-Owner or renter, so it's okay
    Ok(())
}

pub fn get_speed_on_water_type(
    nautical_speed: &Vec<MovementSpeed>,
    mut water_body_type: u8,
    water_depth: Option<i16>,
    is_player: bool,
) -> f32 {
    if is_player && water_depth.is_some() && water_depth.unwrap() < PLAYER_MIN_SWIM_DEPTH {
        water_body_type = SurfaceType::Ground as u8;
    }
    match nautical_speed.iter().find(|ns| ns.surface_type as u8 == water_body_type) {
        Some(ns) => ns.speed,
        None => 0.0,
    }
}
