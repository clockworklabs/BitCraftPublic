use spacetimedb::ReducerContext;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::components::inventory_state;

#[spacetimedb::reducer]
pub fn cheat_remove_item_quantity(ctx: &ReducerContext, inventory_entity_id: u64, pocket_index: i32, quantity_to_remove: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDeleteItem) {
        return Err("Unauthorized.".into());
    }

    let mut inv = ctx.db.inventory_state().entity_id().find(inventory_entity_id).unwrap();
    if inv.get_pocket_contents(pocket_index as usize).is_some() {
        inv.remove_quantity_at(pocket_index as usize, quantity_to_remove);
        ctx.db.inventory_state().entity_id().update(inv);
        Ok(())
    } else {
        Err("Could not find item stack.".into())
    }
}

#[spacetimedb::reducer]
pub fn cheat_remove_item_quantity_all(ctx: &ReducerContext, inventory_entity_id: u64, pocket_index: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDeleteItem) {
        return Err("Unauthorized.".into());
    }

    let mut inv = ctx.db.inventory_state().entity_id().find(inventory_entity_id).unwrap();
    if inv.get_pocket_contents(pocket_index as usize).is_some() {
        inv.set_at(pocket_index as usize, None);
        ctx.db.inventory_state().entity_id().update(inv);
        Ok(())
    } else {
        Err("Could not find item stack.".into())
    }
}
