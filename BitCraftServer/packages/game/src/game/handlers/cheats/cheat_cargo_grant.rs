use crate::game::discovery::Discovery;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::CheatCargoGrantRequest;
use crate::messages::game_util::ItemStack;
use crate::{inventory_state, player_state, unwrap_or_err, InventoryState};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_cargo_grant(ctx: &ReducerContext, request: CheatCargoGrantRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatCargoGrant) {
        return Err("Unauthorized.".into());
    }

    if ctx.db.player_state().entity_id().find(request.owner_entity_id).is_none() {
        return Err("Invalid player id".into());
    };

    let mut player_inventory = unwrap_or_err!(
        InventoryState::get_player_inventory(ctx, request.owner_entity_id),
        "Player has no inventory"
    );
    let item_stacks = vec![ItemStack::single_cargo(request.cargo_id)];

    player_inventory.add_multiple_with_overflow(ctx, &item_stacks);
    ctx.db.inventory_state().entity_id().update(player_inventory);

    let mut discovery = Discovery::new(request.owner_entity_id);
    discovery.acquire_cargo(ctx, request.cargo_id);
    discovery.commit(ctx);

    player_action_helpers::post_reducer_update_cargo(ctx, request.owner_entity_id);

    Ok(())
}
