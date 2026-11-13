use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::static_data::ToolTypeDesc;
use spacetimedb::{log, ReducerContext};

use crate::game::discovery::Discovery;
use crate::messages::components::{player_state, InventoryState};
use crate::messages::game_util::{ItemStack, ItemType};
use crate::{inventory_state, tool_desc};

#[spacetimedb::reducer]
pub fn cheat_item_stack_grant(
    ctx: &ReducerContext,
    player_entity_id: u64,
    item_id: i32,
    quantity: i32,
    is_cargo: bool,
) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatItemStackGrant) {
        return Err("Unauthorized.".into());
    }

    if ctx.db.player_state().entity_id().find(player_entity_id).is_none() {
        return Err(format!("Cannot find player {} in current region", player_entity_id));
    }

    let item_stack = ItemStack::new(ctx, item_id, if is_cargo { ItemType::Cargo } else { ItemType::Item }, quantity);

    reduce(ctx, player_entity_id, item_stack)
}

pub fn reduce(ctx: &ReducerContext, player_entity_id: u64, item_stack: ItemStack) -> Result<(), String> {
    let mut discovery = Discovery::new(player_entity_id);

    if !InventoryState::add_and_discover(ctx, player_entity_id, &mut discovery, item_stack, false) {
        return Err("Inventory is full.".into());
    }

    discovery.commit(ctx);

    Ok(())
}

#[spacetimedb::reducer]
pub fn cheat_item_stack_grant_and_equip(
    ctx: &ReducerContext,
    player_entity_id: u64,
    item_id: i32,
    quantity: i32,
    is_cargo: bool,
) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatItemStackGrant) {
        return Err("Unauthorized.".into());
    }

    let item_stack = ItemStack::new(ctx, item_id, if is_cargo { ItemType::Cargo } else { ItemType::Item }, quantity);

    let mut discovery = Discovery::new(player_entity_id);
    discovery.acquire_item_stack(ctx, &item_stack);
    discovery.commit(ctx);

    let mut toolbelt_inventory = InventoryState::get_player_toolbelt(ctx, player_entity_id).unwrap();
    if let Some(tool) = ctx.db.tool_desc().item_id().filter(item_stack.item_id).next() {
        let tool_pocket = ToolTypeDesc::get_slot_from_tool_type_id(ctx, tool.tool_type) as usize;
        toolbelt_inventory.set_at(tool_pocket, Some(item_stack));
        ctx.db.inventory_state().entity_id().update(toolbelt_inventory);
    } else {
        log::error!("Unknown tool for item_id {}", item_stack.item_id);
    }

    Ok(())
}
