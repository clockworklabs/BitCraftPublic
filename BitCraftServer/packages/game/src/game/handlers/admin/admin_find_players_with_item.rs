use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{
            claim_member_state, deployable_state_v2, dropped_inventory_state, inventory_state, player_housing_state, player_state,
            player_username_state, InventoryState, PlayerHousingState,
        },
        game_util::ItemType,
    },
};

//TODO Scan player housing, open and closed trades

#[spacetimedb::reducer]
pub fn admin_find_all_players_with_item(ctx: &ReducerContext, item_id: i32, is_cargo: bool, claim_entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }
    if claim_entity_id > 0 {
        for member in ctx.db.claim_member_state().claim_entity_id().filter(claim_entity_id) {
            let player_entity_id = member.player_entity_id;
            log_quantity(ctx, item_id, is_cargo, player_entity_id, None);
        }
    } else {
        for player in ctx.db.player_state().iter() {
            log_quantity(ctx, item_id, is_cargo, player.entity_id, None);
        }
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_find_all_players_with_item_above_quantity(
    ctx: &ReducerContext,
    item_id: i32,
    is_cargo: bool,
    claim_entity_id: u64,
    min_quantity: u64,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }
    if claim_entity_id > 0 {
        for member in ctx.db.claim_member_state().claim_entity_id().filter(claim_entity_id) {
            let player_entity_id = member.player_entity_id;
            log_quantity(ctx, item_id, is_cargo, player_entity_id, Some(min_quantity));
        }
    } else {
        for player in ctx.db.player_state().iter() {
            log_quantity(ctx, item_id, is_cargo, player.entity_id, Some(min_quantity));
        }
    }
    Ok(())
}

pub fn log_quantity(ctx: &ReducerContext, item_id: i32, is_cargo: bool, player_entity_id: u64, min_quantity: Option<u64>) {
    let item_type = if is_cargo { ItemType::Cargo } else { ItemType::Item };

    // player inventories
    let mut inventory_qty = 0;
    for inventory in ctx.db.inventory_state().owner_entity_id().filter(player_entity_id) {
        inventory_qty += quantity_of_item(&inventory, item_id, item_type)
    }

    // deployable inventories
    let mut deployable_qty = 0;
    for deployable in ctx.db.deployable_state_v2().owner_id().filter(player_entity_id) {
        if let Some(inventory) = ctx.db.inventory_state().owner_entity_id().filter(deployable.entity_id).next() {
            deployable_qty += quantity_of_item(&inventory, item_id, item_type)
        }
    }

    // items in bank
    let mut bank_qty = 0;
    for inventory in ctx.db.inventory_state().player_owner_entity_id().filter(player_entity_id) {
        bank_qty += quantity_of_item(&inventory, item_id, item_type)
    }

    // items in housing
    let mut housing_qty = 0;
    if let Some(player_housing) = ctx.db.player_housing_state().entity_id().find(player_entity_id) {
        for inventory in PlayerHousingState::get_all_player_housing_inventories(ctx, player_housing.network_entity_id) {
            housing_qty += quantity_of_item(&inventory, item_id, item_type)
        }
    }

    // owned item piles
    let mut item_pile_qty = 0;
    for dropped_inventory_state in ctx.db.dropped_inventory_state().owner_entity_id().filter(player_entity_id) {
        let inventory = dropped_inventory_state.inventory(ctx);
        item_pile_qty += quantity_of_item(&inventory, item_id, item_type)
    }

    let total_qty = inventory_qty + deployable_qty + bank_qty + housing_qty;
    if let Some(threshold) = min_quantity {
        if total_qty < threshold {
            return;
        }
    }
    if total_qty > 0 {
        let username = ctx.db.player_username_state().entity_id().find(player_entity_id).unwrap().username;
        log::info!("Player {} (entity id {}) has item_id {}", username, player_entity_id, item_id);
        log::info!("In inventories: {}", inventory_qty);
        log::info!("In banks: {}", bank_qty);
        log::info!("In deployables: {}", deployable_qty);
        log::info!("In owned dropped item piles: {}", item_pile_qty);
        log::info!("total quantity: {}", total_qty);
    }
}

pub fn quantity_of_item(inventory: &InventoryState, item_id: i32, item_type: ItemType) -> u64 {
    let mut quantity = 0;
    for stack in inventory.get_all_content() {
        if stack.item_type == item_type && stack.item_id == item_id {
            quantity += stack.quantity as u64
        }
    }
    quantity
}
