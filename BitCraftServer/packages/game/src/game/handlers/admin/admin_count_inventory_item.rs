use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{player_username_state, InventoryState},
        game_util::ItemType,
    },
};
use spacetimedb::{ReducerContext, Table};
use std::collections::HashMap;
use std::fmt::Write;

#[spacetimedb::reducer]
pub fn admin_count_inventory_items(ctx: &ReducerContext, item_id: i32, limit: u32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }
    // Mapping from player username to total item quantity
    let mut player_item_counts: HashMap<String, i32> = HashMap::new();

    // Iterate over all player states
    for player_username_state in ctx.db.player_username_state().iter() {
        let entity_id = player_username_state.entity_id;

        // Retrieve inventory for the player
        if let Some(inventory) = InventoryState::get_player_inventory(ctx, entity_id) {
            // Sum up quantities of matching items
            let total_quantity: i32 = inventory
                .pockets
                .iter()
                .filter(|pocket| {
                    if let Some(item_stack) = pocket.contents {
                        return item_stack.item_id == item_id && matches!(item_stack.item_type, ItemType::Item);
                    }
                    false
                })
                .map(|item_stack| item_stack.contents.unwrap().quantity)
                .sum();

            if total_quantity > 0 {
                player_item_counts.insert(player_username_state.username, total_quantity);
            }
        }
    }

    // Convert to a vec and sort by decreasing quantity
    let mut sorted_players: Vec<_> = player_item_counts.into_iter().collect();
    sorted_players.sort_by_key(|b| std::cmp::Reverse(b.1)); // sort descending

    // Print up to the specified limit
    let mut output = String::new();
    writeln!(&mut output, "Total players that have item: {}", sorted_players.len()).unwrap();
    for (i, (username, quantity)) in sorted_players.iter().take(limit as usize).enumerate() {
        writeln!(&mut output, "{}. Player {}: {}", i + 1, username, quantity).unwrap();
    }

    Err(output)
}
