use crate::{
    barter_stall_state, collectible_desc, deployable_collectible_state, deployable_desc, deployable_state_v2,
    game::{
        claim_helper,
        coordinates::*,
        dimensions,
        discovery::Discovery,
        game_state::{
            self,
            game_state_filters::{coordinates_any, coordinates_any_float},
            insert_location_float,
        },
        terrain_chunk::TerrainChunkCache,
    },
    inventory_state,
    messages::{
        components::{dimension_description_state, dimension_network_state, portal_state, PlayerHousingState},
        empire_shared::{empire_player_data_state, empire_state},
        game_util::ItemStack,
        static_data::{CollectibleType, DeployableAppearanceOverrideDesc, DeployableDesc, DeployableType},
    },
    mobile_entity_state, mounting_state, pathfinding_desc, unwrap_or_err, vault_state, BarterStallState, DroppedInventoryState,
    FootprintTileState, FootprintType, InventoryState, MobileEntityState, MovementType, PlayerActionState, PlayerActionType,
    PlayerTimestampState,
};
use spacetimedb::{ReducerContext, Table};

use crate::messages::components::{DeployableStateV2, HealthState, PlayerState};
use crate::messages::static_data::deployable_appearance_override_desc;

impl DeployableAppearanceOverrideDesc {
    pub fn affected_models(&self) -> &[String] {
        match self.affected_model_addresses.as_deref() {
            Some(addresses) if !addresses.is_empty() => addresses,
            _ => std::slice::from_ref(&self.affected_model_address),
        }
    }

    pub fn affects_model(&self, model_address: &str) -> bool {
        self.affected_models().iter().any(|address| address == model_address)
    }

    pub fn affects_any_same_model_as(&self, other: &Self) -> bool {
        self.affected_models().iter().any(|address| other.affects_model(address))
    }
}

pub fn get_active_deployable_appearance_override(
    ctx: &ReducerContext,
    actor_id: u64,
    model_address: String,
) -> Option<DeployableAppearanceOverrideDesc> {
    let vault = ctx.db.vault_state().entity_id().find(&actor_id)?;
    for collectible in vault.collectibles {
        if !collectible.activated {
            continue;
        }

        let collectible_desc = ctx.db.collectible_desc().id().find(&collectible.id)?;
        if collectible_desc.collectible_type != CollectibleType::DeployableAppearanceOverride {
            continue;
        }

        let appearance_override = ctx
            .db
            .deployable_appearance_override_desc()
            .collectible_id()
            .find(&collectible.id)?;
        if appearance_override.affects_model(&model_address) {
            return Some(appearance_override);
        }
    }

    None
}

pub fn has_deployed_deployable_with_model_address(ctx: &ReducerContext, actor_id: u64, model_address: &str) -> bool {
    for deployable in ctx.db.deployable_state_v2().owner_id().filter(actor_id) {
        let Some(deployable_collectible) = ctx
            .db
            .deployable_collectible_state()
            .deployable_entity_id()
            .find(&deployable.entity_id)
        else {
            continue;
        };

        if deployable_collectible.location.is_none() {
            continue;
        }

        let Some(deployable_desc) = ctx.db.deployable_desc().id().find(&deployable.deployable_description_id) else {
            continue;
        };

        if deployable_desc.model_address == model_address {
            return true;
        }
    }

    false
}

pub fn deploy_deployable(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    collectible_id: i32,
    direction: i32,
    coord: SmallHexTile,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let collectible_desc = unwrap_or_err!(ctx.db.collectible_desc().id().find(collectible_id), "Collectible does not exist.");

    for knowledge_req in &collectible_desc.required_knowledges_to_use {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *knowledge_req) {
            return Err("You don't meet the knowledge requirements to deploy this collectible".into());
        }
    }

    let deployable_description = unwrap_or_err!(
        ctx.db.deployable_desc().deploy_from_collectible_id().find(collectible_id),
        "This is not a deployable."
    );

    // DAB Note: this only works if there is only a maximum of 1 deployable per collectible type.
    // If we want to deploy more than one, we will need to get rid of the CollectibleType::Deployable and only use DeployableCollectibleState entries in the UI.
    let mut deployable_collectible = unwrap_or_err!(
        ctx.db
            .deployable_collectible_state()
            .owner_entity_id()
            .filter(actor_id)
            .filter(|v| v.collectible_id == collectible_id)
            .next(),
        "You don't own a deployable of that type"
    );

    if deployable_collectible.location.is_some() {
        return Err("Deployable is already deployed".into());
    }

    let vault = ctx.db.vault_state().entity_id().find(&actor_id).unwrap();
    if !vault
        .collectibles
        .iter()
        .any(|c| c.id == deployable_description.deploy_from_collectible_id)
    {
        return Err("You don't own that kind of deployable.".into());
    }

    if ctx
        .db
        .deployable_collectible_state()
        .owner_entity_id()
        .filter(actor_id)
        .filter_map(|v| match v.location {
            Some(_) => Some(v.deployable_desc_id),
            None => None,
        })
        .filter(|id: &i32| ctx.db.deployable_desc().id().find(id).unwrap().deployable_type == deployable_description.deployable_type)
        .next()
        .is_some()
    {
        let deployable_type = deployable_description.deployable_type;
        return Err(format!("You can only deploy one {{0}}.|~{deployable_type}"));
    }

    let player_coordinates = coordinates_any_float(ctx, actor_id);
    if player_coordinates.dimension != dimensions::OVERWORLD {
        if !deployable_description.can_enter_portals {
            return Err("This deployable can't be deployed in interiors".into());
        }

        let dimension_network_entity_id = ctx
            .db
            .dimension_description_state()
            .dimension_id()
            .find(player_coordinates.dimension)
            .unwrap()
            .dimension_network_entity_id;
        let building_entity_id = ctx
            .db
            .dimension_network_state()
            .entity_id()
            .find(dimension_network_entity_id)
            .unwrap()
            .building_id;
        if !ctx
            .db
            .portal_state()
            .target_building_entity_id()
            .filter(building_entity_id)
            .any(|ps| ps.allow_deployables)
            && PlayerHousingState::from_dimension(ctx, player_coordinates.dimension).is_none()
        {
            return Err("You can't deploy deployables in this interior".into());
        }
    }

    let player_small_hex_tile = player_coordinates.parent_small_tile();

    if player_small_hex_tile.distance_to(coord) > 2 {
        return Err("Too far".into());
    }

    let target_large_hex_tile = coord.parent_large_tile();
    let target_coordinates = coord;

    let terrain_tile = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &target_large_hex_tile),
        "Could not find target cell"
    );
    let allow_ground = deployable_description.placeable_on_land;

    if terrain_tile.is_submerged() {
        let water_depth = terrain_tile.water_depth() as i32;
        let pathfinding = unwrap_or_err!(
            ctx.db.pathfinding_desc().id().find(&deployable_description.pathfinding_id),
            "Invalid pathfinding info"
        );

        if pathfinding.min_water_depth > water_depth {
            return Err("The water level is too shallow.".into());
        }

        if water_depth > pathfinding.max_water_depth {
            return Err("The water level is too deep.".into());
        }
    }

    if !terrain_tile.is_submerged() && !allow_ground {
        return Err("This needs to be deployed in water".into());
    }

    for footprint in FootprintTileState::get_at_location(ctx, &coord) {
        if footprint.footprint_type == FootprintType::Hitbox || footprint.footprint_type == FootprintType::Walkable {
            return Err("Placement location is obstructed".into());
        }
    }

    if !dry_run {
        let offset = OffsetCoordinatesSmall::from(target_coordinates);
        let deployable_entity_id = deployable_collectible.deployable_entity_id;
        let appearance_override_id = get_active_deployable_appearance_override(ctx, actor_id, deployable_description.model_address)
            .map(|appearance_override| appearance_override.id)
            .unwrap_or(0);

        let mut deployable = ctx.db.deployable_state_v2().entity_id().find(&deployable_entity_id).unwrap();
        deployable.direction = direction;
        deployable.claim_entity_id = claim_helper::get_claim_on_tile(ctx, target_coordinates)
            .map(|x| x.claim_id)
            .unwrap_or(0);
        deployable.appearance_override_id = appearance_override_id;
        ctx.db.deployable_state_v2().entity_id().update(deployable);

        deployable_collectible.location = Some(offset);
        ctx.db
            .deployable_collectible_state()
            .deployable_entity_id()
            .update(deployable_collectible);

        let spawn_offset_float = OffsetCoordinatesFloat::from(offset);
        game_state::insert_location_float(ctx, deployable_entity_id, spawn_offset_float);

        // DAB Note: for now, we will create/delete inventories. But in the future we might keep the inventory state alive yet empty.
        if !InventoryState::new(
            ctx,
            deployable_description.storage + deployable_description.stockpile,
            deployable_description.item_slot_size,
            deployable_description.cargo_slot_size,
            deployable_description.storage,
            deployable_entity_id,
            0,
            None,
        ) {
            return Err("Failed to insert deployable stockpile inventory".into());
        }

        if deployable_description.barter > 0 {
            let barter_stall_state = BarterStallState {
                entity_id: deployable_entity_id,
                market_mode_enabled: false,
            };

            ctx.db.barter_stall_state().try_insert(barter_stall_state)?;
        }

        let mut discovery = Discovery::new(actor_id);
        discovery.acquire_deployable(ctx, deployable_description.id);
        discovery.commit(ctx);
    }

    Ok(())
}

pub fn deploy_standalone_deployable(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    deployable_entity_id: u64,
    deployable_id: i32,
    direction: i32,
    coord: SmallHexTile,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if ctx.db.mounting_state().entity_id().find(&actor_id).is_some() {
        return Err("Can't place a deployable while in a deployable".into());
    }

    let deployable_description = unwrap_or_err!(ctx.db.deployable_desc().id().find(&deployable_id), "This is not a deployable.");

    let player_coordinates = coordinates_any_float(ctx, actor_id);
    if player_coordinates.dimension != dimensions::OVERWORLD {
        return Err("You cannot deploy deployables in interiors.".into());
    }

    let player_small_hex_tile = player_coordinates.parent_small_tile();

    if player_small_hex_tile.distance_to(coord) > 2 {
        return Err("Too far".into());
    }

    let target_large_hex_tile = coord.parent_large_tile();
    let target_coordinates = coord;

    let terrain_tile = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &target_large_hex_tile),
        "Could not find target cell"
    );
    if terrain_tile.is_submerged() {
        if deployable_description.movement_type != MovementType::Water && deployable_description.movement_type != MovementType::Amphibious {
            return Err("This needs to be deployed on ground".into());
        }
    } else {
        if deployable_description.movement_type == MovementType::Water {
            return Err("This needs to be deployed in water".into());
        }
    }

    for footprint in FootprintTileState::get_at_location(ctx, &coord) {
        if footprint.footprint_type == FootprintType::Hitbox || footprint.footprint_type == FootprintType::Walkable {
            return Err("Placement location is obstructed".into());
        }
    }

    let player_empire_data = unwrap_or_err!(
        ctx.db.empire_player_data_state().entity_id().find(&actor_id),
        "Player is not in an empire"
    );
    let empire = unwrap_or_err!(
        ctx.db.empire_state().entity_id().find(&player_empire_data.empire_entity_id),
        "Empire doesn't exist"
    );

    // Add deployable
    let offset = OffsetCoordinatesSmall::from(target_coordinates);

    let deployable_description_id = deployable_description.id;
    let entity_id = deployable_entity_id;
    let deployable_state = DeployableStateV2 {
        entity_id,
        deployable_description_id,
        owner_id: 0,
        claim_entity_id: claim_helper::get_claim_on_tile(ctx, target_coordinates)
            .map(|x| x.claim_id)
            .unwrap_or(0),
        direction,
        appearance_override_id: 0,
        nickname: format!("{}'s {}", empire.name, deployable_description.name),
        hidden: false,
    };

    if !dry_run {
        if ctx.db.deployable_state_v2().try_insert(deployable_state).is_err() {
            return Err("Failed to insert deployable".into());
        }

        let spawn_offset_float = OffsetCoordinatesFloat::from(offset);
        insert_location_float(ctx, entity_id, spawn_offset_float);

        if deployable_description.storage + deployable_description.stockpile > 0 {
            if !InventoryState::new(
                ctx,
                deployable_description.storage + deployable_description.stockpile,
                deployable_description.item_slot_size,
                deployable_description.cargo_slot_size,
                deployable_description.storage,
                entity_id,
                0,
                None,
            ) {
                return Err("Failed to insert deployable stockpile inventory".into());
            }
        }

        let mut discovery = Discovery::new(actor_id);
        discovery.acquire_deployable(ctx, deployable_description_id);
        discovery.commit(ctx);
    }

    Ok(())
}

pub fn dismount_deployable_and_explore(
    ctx: &ReducerContext,
    player_entity_id: u64,
    target_coordinates: FloatHexTile,
    skip_deployable_icon: bool,
) -> Result<(), String> {
    // move player off deployable
    PlayerState::move_player_and_explore(ctx, player_entity_id, &target_coordinates, &target_coordinates, 0.0, false, None)?;
    dismount_deployable(ctx, player_entity_id, skip_deployable_icon);

    Ok(())
}

pub fn dismount_deployable_and_explore_and_set_deployable_position(
    ctx: &ReducerContext,
    player_entity_id: u64,
    target_coordinates: FloatHexTile,
    deployable_coordinates: FloatHexTile,
    skip_deployable_icon: bool,
) -> Result<(), String> {
    // move player off deployable
    PlayerState::move_player_and_explore(ctx, player_entity_id, &target_coordinates, &target_coordinates, 0.0, false, None)?;
    dismount_deployable_and_set_deployable_position(ctx, player_entity_id, skip_deployable_icon, deployable_coordinates);

    Ok(())
}

pub fn dismount_deployable(ctx: &ReducerContext, player_entity_id: u64, skip_deployable_icon: bool) {
    if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&player_entity_id) {
        // deployable icon update might be skipped when we move a deployable off-claim, in which case the icon is updated prior the resolving of the dismount transaction
        if !skip_deployable_icon {
            if let Some(mut deployable_collectible) = ctx
                .db
                .deployable_collectible_state()
                .deployable_entity_id()
                .find(&mounting.deployable_entity_id)
            {
                let coord = coordinates_any(ctx, mounting.deployable_entity_id);
                deployable_collectible.location = Some(coord.into());
                ctx.db
                    .deployable_collectible_state()
                    .deployable_entity_id()
                    .update(deployable_collectible);
            }
        }

        ctx.db.mounting_state().entity_id().delete(&player_entity_id);
    }
}

pub fn dismount_deployable_and_set_deployable_position(
    ctx: &ReducerContext,
    player_entity_id: u64,
    skip_deployable_icon: bool,
    deployable_coordinates: FloatHexTile,
) {
    if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&player_entity_id) {
        // deployable icon update might be skipped when we move a deployable off-claim, in which case the icon is updated prior the resolving of the dismount transaction
        if !skip_deployable_icon {
            if let Some(mut deployable_collectible) = ctx
                .db
                .deployable_collectible_state()
                .deployable_entity_id()
                .find(&mounting.deployable_entity_id)
            {
                let coord = deployable_coordinates.parent_small_tile();
                deployable_collectible.location = Some(coord.into());
                ctx.db
                    .deployable_collectible_state()
                    .deployable_entity_id()
                    .update(deployable_collectible);

                let new_location =
                    MobileEntityState::for_location(mounting.deployable_entity_id, deployable_coordinates.into(), ctx.timestamp);

                ctx.db.mobile_entity_state().entity_id().update(new_location);
            }
        }
        // delete player mounting
        ctx.db.mounting_state().entity_id().delete(&player_entity_id);

        //refresh stats
        PlayerState::collect_stats(ctx, player_entity_id);
    }
}

pub fn expel_passengers(ctx: &ReducerContext, deployable_entity_id: u64, skip_deployable_icon: bool, expel_driver: bool) {
    // Expel passengers if the deployable is not already stored (hide_deployable timed reducer happening on sign out)
    if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(&deployable_entity_id) {
        let coordinates = location.coordinates_float();
        let passengers = ctx.db.mounting_state().deployable_entity_id().filter(deployable_entity_id);
        for passenger in passengers {
            if !expel_driver && passenger.deployable_slot == 0 {
                continue;
            }
            dismount_deployable_and_set_deployable_position(ctx, passenger.entity_id, skip_deployable_icon, coordinates);
        }
    }
}

pub fn store_deployable(ctx: &ReducerContext, actor_id: u64, deployable_entity_id: u64, dry_run: bool) -> Result<(), String> {
    let deployable_state = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(&deployable_entity_id),
        "Deployable doesn't exist."
    );

    let deployable_desc = ctx
        .db
        .deployable_desc()
        .id()
        .find(deployable_state.deployable_description_id)
        .unwrap();
    if deployable_desc.deployable_type == DeployableType::SiegeEngine {
        return Err("You cannot store siege engines".into());
    }

    deactivate_deployable_collectible(ctx, actor_id, &deployable_desc, dry_run)?;

    if !dry_run {
        return expel_and_despawn(ctx, actor_id, deployable_entity_id, deployable_desc);
    }
    Ok(())
}

pub fn expel_and_despawn(
    ctx: &ReducerContext,
    actor_id: u64,
    deployable_entity_id: u64,
    deployable_desc: DeployableDesc,
) -> Result<(), String> {
    if deployable_desc.deployable_type == DeployableType::SiegeEngine {
        return Err("You cannot store siege engines".into());
    }

    if deployable_desc.barter > 0 {
        ctx.db.barter_stall_state().entity_id().delete(&deployable_entity_id);
    }

    expel_passengers(ctx, deployable_entity_id, false, true);

    // Expel cargo
    if let Some(inventory) = InventoryState::get_by_owner(ctx, deployable_entity_id) {
        if let Some(mes) = ctx.db.mobile_entity_state().entity_id().find(deployable_entity_id) {
            let deployable_coordinates = mes.coordinates();
            let dropped_items: Vec<ItemStack> = inventory.pockets.iter().filter_map(|p| p.contents).collect();
            DroppedInventoryState::update_from_items(ctx, actor_id, deployable_coordinates.into(), dropped_items, None);
        } else {
            spacetimedb::log::error!("Missing MobileEntityState for deployable {}", deployable_entity_id);
            return Err("Missing MobileEntityState".into());
        }
        ctx.db.inventory_state().entity_id().delete(inventory.entity_id);
    } else {
        // As the point of this function is to delete the inventory state anyway
        // while this is an error case, we log the error but continue processing.
        spacetimedb::log::error!("Missing InventoryState for deployable {}", deployable_entity_id);
    }

    despawn(ctx, deployable_entity_id);

    return Ok(());
}

pub fn deactivate_deployable_collectible(
    ctx: &ReducerContext,
    actor_id: u64,
    deployable_desc: &DeployableDesc,
    dry_run: bool,
) -> Result<(), String> {
    // Deactivate deployable collectible
    let mut vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(&actor_id), "Player has no vault");
    let collectible_id = deployable_desc.deploy_from_collectible_id;
    let collectible = unwrap_or_err!(
        vault.collectibles.iter_mut().find(|c| c.id == collectible_id),
        "You don't own the right collectible"
    );

    if !dry_run {
        collectible.activated = false;
        ctx.db.vault_state().entity_id().update(vault);
    }
    Ok(())
}

pub fn despawn(ctx: &ReducerContext, deployable_entity_id: u64) {
    ctx.db.mobile_entity_state().entity_id().delete(&deployable_entity_id);
    if let Some(mut deployable_collectible) = ctx
        .db
        .deployable_collectible_state()
        .deployable_entity_id()
        .find(&deployable_entity_id)
    {
        if deployable_collectible.location.is_some() {
            deployable_collectible.location = None;
            ctx.db
                .deployable_collectible_state()
                .deployable_entity_id()
                .update(deployable_collectible);
        }
    }

    if let Some(mut deployable_state) = ctx.db.deployable_state_v2().entity_id().find(&deployable_entity_id) {
        if deployable_state.claim_entity_id != 0 {
            deployable_state.claim_entity_id = 0;
            ctx.db.deployable_state_v2().entity_id().update(deployable_state);
        }
    }
}

pub fn is_spawned(ctx: &ReducerContext, deployable_entity_id: u64) -> bool {
    if let Some(deployable_collectible) = ctx
        .db
        .deployable_collectible_state()
        .deployable_entity_id()
        .find(&deployable_entity_id)
    {
        return deployable_collectible.location.is_some();
    }
    false
}

pub fn move_deployable(
    ctx: &ReducerContext,
    deployable_entity_id: u64,
    origin: OffsetCoordinatesFloat,
    destination: OffsetCoordinatesFloat,
    timestamp: u64,
    duration: f32,
) -> Result<(), String> {
    let mut new_location = MobileEntityState::for_location(
        deployable_entity_id,
        (origin.x, origin.z, destination.dimension).into(),
        ctx.timestamp,
    );
    new_location.timestamp = timestamp;
    ctx.db.mobile_entity_state().entity_id().update(new_location);

    //TODO: Update the claim_entity_id column once we add a mobile trader stand
    let source_coordinates: FloatHexTile = origin.into();
    let target_coordinates: FloatHexTile = destination.into();

    // passengers move by direction offset
    // TODO: We will need to rework this with offset movement. Player location should be deployable position + offset based on local position.
    let action_type = if source_coordinates == target_coordinates {
        PlayerActionType::None
    } else {
        PlayerActionType::DeployableMove
    };
    let duration = (duration * 1000.0) as u64;
    for passenger_id in DeployableStateV2::passengers_iter(ctx, deployable_entity_id) {
        PlayerTimestampState::refresh(ctx, passenger_id, ctx.timestamp);
        PlayerState::move_player_and_explore(
            ctx,
            passenger_id,
            &source_coordinates,
            &target_coordinates,
            0.0,
            false,
            Some(timestamp),
        )?;
        PlayerActionState::success(
            ctx,
            passenger_id,
            action_type,
            PlayerActionType::DeployableMove.get_layer(ctx),
            duration,
            None,
            None,
            timestamp,
        );
    }
    Ok(())
}
