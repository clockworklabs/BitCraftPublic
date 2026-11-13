use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::{active_buff_state, player_state, starving_player_state, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_clear_buffs_and_debuffs(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClearBuffsAndDebuffs) {
        return Err("Unauthorized.".into());
    }

    if ctx.db.player_state().entity_id().find(player_entity_id).is_none() {
        return Err("Invalid player id".into());
    };

    let mut active_buff_state = unwrap_or_err!(
        ctx.db.active_buff_state().entity_id().find(player_entity_id),
        "Unable to get ActiveBuffState"
    );

    active_buff_state.remove_all_active_buffs(ctx);
    ctx.db.active_buff_state().entity_id().update(active_buff_state);
    ctx.db.starving_player_state().entity_id().delete(player_entity_id);

    Ok(())
}
