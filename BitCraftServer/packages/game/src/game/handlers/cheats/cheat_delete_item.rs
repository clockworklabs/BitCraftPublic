use crate::{
    game::handlers::cheats::cheat_type::{can_run_cheat, CheatType},
    inventory_state,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_delete_item(ctx: &ReducerContext, inventory_entity_id: u64, pocket_index: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatDeleteItem) {
        return Err("Unauthorized.".into());
    }

    let mut inv = ctx.db.inventory_state().entity_id().find(inventory_entity_id).unwrap();
    inv.pockets[pocket_index as usize].contents = None;
    ctx.db.inventory_state().entity_id().update(inv);
    Ok(())
}
