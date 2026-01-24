use spacetimedb::{ReducerContext, Table};

use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::messages::components::*;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::static_data::{QuestChainDesc, quest_chain_desc, quest_stage_desc, stage_rewards_desc};
use crate::{unwrap_or_err};

#[spacetimedb::reducer]
pub fn complete_quest_chain(ctx: &ReducerContext, id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let quest_chain_desc = unwrap_or_err!(
        ctx.db.quest_chain_desc()
        .id()
        .find(&id),
        "Failed to find quest chain description."
    );

    // Hints don't need to be started, they just trigger and complete instantly.
    if quest_chain_desc.is_hint {
        start_quest_chain_internal(ctx, actor_id, &quest_chain_desc)?;
    }

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&actor_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == id),
        "Cannot complete quest. Quest not started."
    );

    let is_onboarding = quest_chain_desc.id == 1;
    if !quest_chain_desc.is_hint && quest_chain_state.stage_id != -1 && !is_onboarding {
        return Err("Cannot complete quest. Not on hand-in stage.".into());
    }

    quest_chain_state.is_active = false;
    quest_chain_state.completed = true;
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);

    if !quest_chain_desc.is_hint {
        quest_chain_desc.give_rewards(ctx, actor_id)?;
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn start_quest_chain(ctx: &ReducerContext, id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let quest_chain_desc = unwrap_or_err!(
        ctx.db.quest_chain_desc()
        .id()
        .find(&id),
        "Failed to find quest chain description."
    );

    start_quest_chain_internal(ctx, actor_id, &quest_chain_desc)
}

fn start_quest_chain_internal(ctx: &ReducerContext, actor_id : u64, desc : &QuestChainDesc) -> Result<(), String> {
    if !desc.is_hint {
        desc.check_requirements(ctx, actor_id)?;
    }

    let quest_chain_state_option = 
    ctx.db.quest_chain_state()
    .player_entity_id()
    .filter(&actor_id)
    .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == desc.id);

    if quest_chain_state_option.is_none(){
        ctx.db.quest_chain_state().try_insert(QuestChainState{
            entity_id: game_state::create_entity(ctx),
            player_entity_id: actor_id,
            quest_chain_desc_id: desc.id,
            stage_id: desc.stages.first().copied().unwrap_or(0),
            is_active: false,
            completed: false,
            stage_rewards_awarded: Vec::new(),
        })?;
    }

    if !desc.is_hint {
        // Starting a new quest makes it active immediately.
        set_quest_chain_active_internal(ctx, actor_id, desc.id)?;
    }


    Ok(())
}

#[spacetimedb::reducer]
pub fn advance_quest_stage(ctx: &ReducerContext, chain_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&actor_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == chain_id),
        "Cannot advance quest. Quest not started."
    );

    // Already on the hand-in stage, don't advance.
    if quest_chain_state.stage_id == -1 {
        return Ok(());
    }

    let quest_chain_desc = unwrap_or_err!(
        ctx.db.quest_chain_desc().id().find(chain_id), "Cannot advance quest. Cannot find quest chain."
    );

    // Find the stage we're on. If it's invalid, flip the player to the first stage in this quest (in case we delete a stage)
    let mut current_stage_id = quest_chain_state.stage_id;
    let mut quest_stage_option = ctx.db.quest_stage_desc().id().find(current_stage_id);
    if quest_stage_option.is_none() {
        current_stage_id = unwrap_or_err!(quest_chain_desc.stages.first().copied(), "Cannot advance quest. Quest chain does not have a first stage.");
        quest_stage_option = ctx.db.quest_stage_desc().id().find(current_stage_id);
    }
    let quest_stage = unwrap_or_err!(quest_stage_option, "Cannot advance quest. Current stage {{0}} in chain {{1}} invalid.|~{}|~{}", current_stage_id, chain_id);

    quest_stage.fulfil_completion_conditions(ctx, actor_id)?;

    // Let it stay at -1 if this is the last stage. -1 represents hand-in stage.
    let mut new_stage_id = -1;
    if let Some(mut stage_index) = quest_chain_desc.stages.iter().position(|&s| s == quest_chain_state.stage_id){
        stage_index += 1;
        if stage_index < quest_chain_desc.stages.len() {
            new_stage_id = quest_chain_desc.stages[stage_index];
        }
    } else {
        return Err(format!("Cannot advance quest. Chain {{0}} doesn't have stage {{1}}.|~{}|~{}", chain_id, quest_chain_state.stage_id));
    }

    quest_chain_state.stage_id = new_stage_id;
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn set_quest_chain_active(ctx: &ReducerContext, id : i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    set_quest_chain_active_internal(ctx, actor_id, id)
}

fn set_quest_chain_active_internal(ctx: &ReducerContext, actor_id : u64, id : i32) -> Result<(), String> {
    for mut chain_state in ctx.db.quest_chain_state().player_entity_id().filter(&actor_id) {
        let activate = id == chain_state.quest_chain_desc_id;
        if activate != chain_state.is_active {
            chain_state.is_active = activate;
            ctx.db.quest_chain_state().entity_id().update(chain_state);
        }
    }
    
    Ok(())
}

#[spacetimedb::reducer]
pub fn request_stage_reward(ctx: &ReducerContext, reward_id : i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let onboarding_reward_desc = unwrap_or_err!(
        ctx.db.stage_rewards_desc()
        .id()
        .find(&reward_id),
        "Cannot find correct reward."
    );

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&actor_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == onboarding_reward_desc.chain_desc_id),
        "Cannot get reward for a quest not started."
    );

    if quest_chain_state.stage_rewards_awarded.contains(&reward_id){
        return Err("Already received reward.".into());
    }

    quest_chain_state.stage_rewards_awarded.push(reward_id);
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);

    // Award the items
    let mut discovery = Discovery::new(actor_id);
    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
    for mut item_stack in onboarding_reward_desc.rewards {
        discovery.acquire_item_stack(ctx, &item_stack);
        item_stack.auto_collect(ctx, &mut discovery, actor_id);
        inventory.add_multiple_with_overflow(ctx, &vec![item_stack]);
    }
    ctx.db.inventory_state().entity_id().update(inventory);
    discovery.commit(ctx);

    Ok(())
}

#[spacetimedb::reducer]
pub fn skip_onboarding(ctx: &ReducerContext) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&actor_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == 1),
        "Cannot complete quest. Onboarding not started."
    );

    if quest_chain_state.completed {
        return Err("Cannot complete quest. Onboarding already complete".into());
    }

    quest_chain_state.is_active = false;
    quest_chain_state.completed = true;
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
    let mut discovery = Discovery::new(actor_id);

    // Flint Tool Bundle
    let mut stack = ItemStack{ item_id: 12000, quantity: 1, item_type: ItemType::Item, durability: None };
    discovery.acquire_item_stack(ctx, &stack);
    stack.auto_collect(ctx, &mut discovery, actor_id);
    inventory.add_multiple_with_overflow(ctx, &vec![stack]);

    // Flint Tool Bundle 2
    stack = ItemStack{ item_id: 134258632, quantity: 1, item_type: ItemType::Item, durability: None };
    discovery.acquire_item_stack(ctx, &stack);
    stack.auto_collect(ctx, &mut discovery, actor_id);
    inventory.add_multiple_with_overflow(ctx, &vec![stack]);

    // 10x Mushroom skewer
    stack = ItemStack{ item_id: 1170001, quantity: 10, item_type: ItemType::Item, durability: None };
    discovery.acquire_item_stack(ctx, &stack);
    stack.auto_collect(ctx, &mut discovery, actor_id);
    inventory.add_multiple_with_overflow(ctx, &vec![stack]);

    ctx.db.inventory_state().entity_id().update(inventory);
    discovery.commit(ctx);

    Ok(())
}