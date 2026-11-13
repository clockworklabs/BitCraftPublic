use crate::{game::handlers::cheats::cheat_type::{CheatType, can_run_cheat}, messages::components::{QuestChainState, quest_chain_state}, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_quest_clear(ctx: &ReducerContext, player_entity_id: u64, quest_desc_id: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClearQuest) {
        return Err("Unauthorized.".into());
    }

    let quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&player_entity_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == quest_desc_id),
        "Cannot clear quest. Quest not started."
    );

    ctx.db.quest_chain_state().entity_id().delete(quest_chain_state.entity_id);
    
    Ok(())
}
