use crate::{game::{game_state, handlers::cheats::cheat_type::{CheatType, can_run_cheat}}, messages::{components::{QuestChainState, quest_chain_state}, static_data::quest_chain_desc}, unwrap_or_err};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
pub fn cheat_quest_restart(ctx: &ReducerContext, player_entity_id: u64, quest_desc_id: i32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender, CheatType::CheatRestartQuest) {
        return Err("Unauthorized.".into());
    }

    let quest_chain_state_option = 
    ctx.db.quest_chain_state()
    .player_entity_id()
    .filter(&player_entity_id)
    .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == quest_desc_id);

    if quest_chain_state_option.is_none(){
        ctx.db.quest_chain_state().try_insert(QuestChainState{
            entity_id: game_state::create_entity(ctx),
            player_entity_id: player_entity_id,
            quest_chain_desc_id: quest_desc_id,
            stage_id: 0,
            is_active: false,
            completed: false,
            stage_rewards_awarded: Vec::new(),
        })?;
    }

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&player_entity_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == quest_desc_id),
        "Cannot complete quest. Quest not started."
    );

    let quest_chain_desc = unwrap_or_err!(
        ctx.db.quest_chain_desc()
        .id()
        .find(quest_desc_id),
        "Failed to find quest chain description."
    );

    quest_chain_state.stage_id = quest_chain_desc.stages.first().copied().unwrap_or(0);
    quest_chain_state.is_active = false;
    quest_chain_state.completed = false;
    quest_chain_state.stage_rewards_awarded = Vec::new();

    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);
    
    Ok(())
}
