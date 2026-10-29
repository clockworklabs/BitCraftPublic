use spacetimedb::ReducerContext;

use crate::{
    building_state, deployable_state_v2,
    game::{entities::{building_state::InventoryState, inventory_type::InventoryType}, permission_helper},
    location_state, loot_chest_state,
    messages::{components::{claim_state, dropped_inventory_state, BuildingState, Permission, PermissionState}, game_util::{ItemStack, ItemType}, static_data::{building_desc, cargo_desc, BuildingCategory}},
    mobile_entity_state, mounting_state, unwrap_or_err, ClaimPermission, DeployableStateV2, SmallHexTile,
};

const MAX_INTERACTION_DISTANCE: i32 = 2;
const MAX_DEPLOYABLE_INTERACTION_DISTANCE: i32 = 10;

fn validate_building(ctx: &ReducerContext, actor_id: u64, player_location: SmallHexTile, building: &BuildingState) -> Result<(), String> {
    // Check building distance from player since the footprint can be massive
    if building.distance_to(ctx, &player_location) > MAX_INTERACTION_DISTANCE {
        return Err("Too far from target inventory".into());
    }

    // Interacting with Barter Stall inventory requires co-owner permission.
    let building_desc = unwrap_or_err!(ctx.db.building_desc().id().find(building.building_description_id), "Unknown building type");
    if building_desc.has_category(ctx,BuildingCategory::Barter) {
        if let Some(claim) = ctx.db.claim_state().entity_id().find(building.claim_entity_id) {
            if !claim.has_co_owner_permissions(ctx, actor_id) {
                return Err("Only claim owners or co-owners can interact with barter stall inventories".into());
            }
        }
    }

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Inventory) {
        return Err("You don't have permission to interact with this buildings inventory".into());
    }

    if !permission_helper::can_interact_with_building(ctx, &building, actor_id, ClaimPermission::Inventory) {
        return Err("You don't have permission to interact with this buildings inventory".into());
    }

    Ok(())
}

fn validate_deployable(
    ctx: &ReducerContext,
    actor_id: u64,
    player_location: SmallHexTile,
    deployable: &DeployableStateV2,
) -> Result<(), String> {
    if deployable.owner_id != actor_id {
        return Err("You don't have permission to interact with this deployable's inventory".into());
    }

    if let Some(mounting_state) = ctx.db.mounting_state().entity_id().find(&actor_id) {
        if mounting_state.deployable_entity_id == deployable.entity_id {
            return Ok(());
        }
    }

    let mobile_entity = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(&deployable.entity_id),
        "Deployable has no location"
    );

    if mobile_entity.coordinates().distance_to(player_location) > MAX_DEPLOYABLE_INTERACTION_DISTANCE {
        return Err("Too far from target inventory".into());
    }

    Ok(())
}

fn validate_tile(ctx: &ReducerContext, actor_id: u64, player_location: SmallHexTile, owner_entity_id: u64) -> Result<(), String> {
    if let Some(location) = get_location(ctx, owner_entity_id) {
        if location.distance_to(player_location) > MAX_INTERACTION_DISTANCE {
            return Err("Too far from target inventory".into());
        }

        if !PermissionState::can_interact_with_tile(ctx, actor_id, location, Permission::Inventory) {
            return Err("You don't have permission to interact with this inventory".into());
        }

        if !permission_helper::can_interact_with_tile(ctx, location, actor_id, ClaimPermission::Inventory) {
            return Err("You don't have permission to interact with this inventory".into());
        }

        return Ok(());
    }

    return Err("This storage is no longer available".into());
}

fn get_location(ctx: &ReducerContext, owner_entity_id: u64) -> Option<SmallHexTile> {
    if let Some(mobile_entity) = ctx.db.mobile_entity_state().entity_id().find(&owner_entity_id) {
        return Some(mobile_entity.coordinates());
    } else if let Some(location) = ctx.db.location_state().entity_id().find(&owner_entity_id) {
        return Some(location.coordinates());
    }

    return None;
}

pub fn validate_interact(
    ctx: &ReducerContext,
    actor_id: u64,
    player_location: SmallHexTile,
    owner_entity_id: u64,
    player_owner_entity_id: u64,
) -> Result<InventoryType, String> {
    if owner_entity_id == actor_id {
        return Ok(InventoryType::Player);
    }

    if player_owner_entity_id != 0 && player_owner_entity_id != actor_id {
        return Err("You don't have permission to interact with this inventory".into());
    }

    if let Some(building) = ctx.db.building_state().entity_id().find(&owner_entity_id) {
        validate_building(ctx, actor_id, player_location, &building)?;
        return Ok(InventoryType::Building);
    }

    if let Some(deployable) = ctx.db.deployable_state_v2().entity_id().find(&owner_entity_id) {
        validate_deployable(ctx, actor_id, player_location, &deployable)?;
        return Ok(InventoryType::Deployable);
    }

    if let Some(_loot_chest) = ctx.db.loot_chest_state().entity_id().find(&owner_entity_id) {
        validate_tile(ctx, actor_id, player_location, owner_entity_id)?;
        return Ok(InventoryType::LootChest);
    }

    if let Some(dropped_inventory) = ctx.db.dropped_inventory_state().entity_id().find(&owner_entity_id) {
        dropped_inventory.validate_interact_and_get_inventory_coordinates(ctx, actor_id)?;
        return Ok(InventoryType::Dropped);
    }
    return Err(format!("Unknown entity type for owner_entity_id {{0}}|~{}", owner_entity_id));
}

pub fn validate_move(target_inventory_type: &InventoryType) -> Result<(), String> {
    if *target_inventory_type == InventoryType::LootChest {
        return Err("Cannot move items into a loot chest".into());
    }
    if *target_inventory_type == InventoryType::Dropped {
        return Err("Cannot move items into a dropped inventory, you have to drop those manually".into());
    }

    return Ok(());
}

pub fn validate_cargo_target(
    ctx: &ReducerContext,
    target_inventory: &InventoryState,
    item_stack: &ItemStack,
) -> Result<(), String> {
    if item_stack.item_type != ItemType::Cargo {
        return Ok(());
    }

    let cargo_desc = unwrap_or_err!(
        ctx.db.cargo_desc().id().find(&item_stack.item_id),
        "Invalid cargo id"
    );

    if cargo_desc.cannot_store_in_buildings
        && ctx.db.building_state().entity_id().find(&target_inventory.owner_entity_id).is_some()
    {
        return Err("You cannot store this cargo in buildings.".into());
    } else if cargo_desc.cannot_store_in_deployables
        && ctx.db.deployable_state_v2().entity_id().find(&target_inventory.owner_entity_id).is_some()
    {
        return Err("You cannot store this cargo in deployables.".into());
    }

    Ok(())
}

pub fn validate_split(source_inventory_type: &InventoryType) -> Result<(), String> {
    if *source_inventory_type == InventoryType::LootChest {
        return Err("Cannot split items inside a loot chest".into());
    }
    if *source_inventory_type == InventoryType::Dropped {
        return Err("Cannot split items inside a dropped inventory".into());
    }
    return Ok(());
}

pub fn validate_swap(source_inventory_type: &InventoryType, target_inventory_type: &InventoryType) -> Result<(), String> {
    if *source_inventory_type == InventoryType::LootChest || *target_inventory_type == InventoryType::LootChest {
        return Err("Cannot swap items with a loot chest".into());
    }
    if *target_inventory_type == InventoryType::Dropped {
        return Err("Cannot swap items into a dropped inventory, you have to drop those manually".into());
    }

    return Ok(());
}

pub fn validate_drop(source_inventory_type: &InventoryType) -> Result<(), String> {
    if *source_inventory_type == InventoryType::Dropped {
        return Err("Cannot drop items from a dropped inventory".into());
    }
    return Ok(());
}
