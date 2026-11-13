use spacetimedb::ReducerContext;

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{
            deployable_state_v2, dropped_inventory_state, inventory_state, player_housing_state, player_lowercase_username_state,
            InventoryState, PlayerHousingState,
        },
        game_util::ItemType,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_delete_all_items_of_type(ctx: &ReducerContext, player_username: String, item_id: i32, is_cargo: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }
    reduce(ctx, player_username, item_id, is_cargo)
}

pub fn reduce(ctx: &ReducerContext, player_username: String, item_id: i32, is_cargo: bool) -> Result<(), String> {
    let player_username_state = unwrap_or_err!(
        ctx.db
            .player_lowercase_username_state()
            .username_lowercase()
            .find(player_username.to_lowercase()),
        "Player not found in current region"
    );

    let player_entity_id = player_username_state.entity_id;

    // Delete items from player
    for inventory in ctx.db.inventory_state().owner_entity_id().filter(player_entity_id) {
        purge_inventory_from(ctx, inventory, item_id, is_cargo);
    }

    // Delete items from deployables
    for deployable in ctx.db.deployable_state_v2().owner_id().filter(player_entity_id) {
        if let Some(inventory) = ctx.db.inventory_state().owner_entity_id().filter(deployable.entity_id).next() {
            purge_inventory_from(ctx, inventory, item_id, is_cargo);
        }
    }

    // Delete items from bank
    for inventory in ctx.db.inventory_state().player_owner_entity_id().filter(player_entity_id) {
        purge_inventory_from(ctx, inventory, item_id, is_cargo);
    }

    // Delete items from player housing
    if let Some(player_housing) = ctx.db.player_housing_state().entity_id().find(player_entity_id) {
        for inventory in PlayerHousingState::get_all_player_housing_inventories(ctx, player_housing.network_entity_id) {
            purge_inventory_from(ctx, inventory, item_id, is_cargo);
        }
    }

    // Delete items from owned item piles
    for dropped_inventory in ctx.db.dropped_inventory_state().owner_entity_id().filter(player_entity_id) {
        if purge_inventory_from(ctx, dropped_inventory.inventory(ctx), item_id, is_cargo)
            && dropped_inventory.inventory(ctx).is_empty() {
                dropped_inventory.delete(ctx);
            }
    }

    Ok(())
}

fn purge_inventory_from(ctx: &ReducerContext, mut inventory: InventoryState, item_id: i32, is_cargo: bool) -> bool {
    let mut modified = false;
    for i in 0..inventory.pockets.len() {
        if let Some(content) = inventory.pockets[i].contents {
            if content.item_id == item_id && (content.item_type == ItemType::Cargo) == is_cargo {
                inventory.set_at(i, None);
                modified = true;
            }
        }
    }
    if modified {
        ctx.db.inventory_state().entity_id().update(inventory);
    }
    modified
}
