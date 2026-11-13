use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::handlers::player::player_set_name;
use crate::player_state;

use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_player_set_name(ctx: &ReducerContext, player_entity_id: u64, name: String) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatUserSetName) {
        return Err("Unauthorized.".into());
    }

    if ctx.db.player_state().entity_id().find(player_entity_id).is_none() {
        return Err("Player doesn't exist.".into());
    }

    player_set_name::reduce(ctx, player_entity_id, name)
}
