use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::messages::components::{action_bar_state, AbilityState};

#[spacetimedb::reducer]
#[feature_gate]
pub fn ability_remove(ctx: &ReducerContext, action_bar_index: u8, local_ability_index: u8) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    reduce(ctx, actor_id, action_bar_index, local_ability_index)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, action_bar_index: u8, local_ability_index: u8) -> Result<(), String> {
    if let Some(action_bar_state) = ctx
        .db
        .action_bar_state()
        .by_player_slot()
        .filter((actor_id, action_bar_index, local_ability_index))
        .next()
    {
        // Remove this action bar entry no matter what
        ctx.db.action_bar_state().entity_id().delete(action_bar_state.entity_id);

        // Remove all unmapped abilities that are expired and not auto-attacks from equipped weapons
        AbilityState::clean_up_unmapped_expired_abilities(ctx, actor_id);
    }

    Ok(())
}
