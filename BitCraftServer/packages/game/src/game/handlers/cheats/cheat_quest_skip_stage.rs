use crate::{game::handlers::cheats::cheat_type::{CheatType, can_run_cheat}, messages::{components::{QuestChainState, quest_chain_state}, static_data::quest_chain_desc}, unwrap_or_err};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_quest_skip_stage(ctx: &ReducerContext, player_entity_id: u64, quest_desc_id: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender, CheatType::CheatSkipQuestStage) {
        return Err("Unauthorized.".into());
    }

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&player_entity_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == quest_desc_id),
        "Cannot advance quest. Quest not started."
    );

    // Already on the hand-in stage, don't advance.
    if quest_chain_state.stage_id == -1 {
        return Ok(());
    }

    let quest_chain_desc = unwrap_or_err!(
        ctx.db.quest_chain_desc().id().find(quest_desc_id), "Cannot advance quest. Cannot find quest chain."
    );

    // Let it stay at -1 if this is the last stage. -1 represents hand-in stage.
    let mut new_stage_id = -1;
    if let Some(mut stage_index) = quest_chain_desc.stages.iter().position(|&s| s == quest_chain_state.stage_id){
        stage_index += 1;
        if stage_index < quest_chain_desc.stages.len() {
            new_stage_id = quest_chain_desc.stages[stage_index];
        }
    } else {
        return Err(format!("Cannot advance quest. Chain {} doesn't have stage {}.", quest_desc_id, quest_chain_state.stage_id));
    }

    quest_chain_state.stage_id = new_stage_id;
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);
    
    Ok(())
}
