use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::messages::components::*;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::static_data::{QuestChainDesc, quest_chain_desc, quest_stage_desc, stage_rewards_desc};
use crate::{unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
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
    
    if quest_chain_state.completed {
        return Err("This quest is already completed".into());
    }

    quest_chain_state.tracked = false;
    quest_chain_state.completed = true;
    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);

    if !quest_chain_desc.is_hint {
        quest_chain_desc.give_rewards(ctx, actor_id)?;
    }

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
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
        
        if desc.unstartable {
            return Err("Cannot start this quest.".into());
        }
    }

    let quest_chain_state_option = 
    ctx.db.quest_chain_state()
    .player_entity_id()
    .filter(&actor_id)
    .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == desc.id);

    if quest_chain_state_option.is_none(){
        if !desc.is_hint {
            untrack_oldest_quest_if_needed(ctx, actor_id)?;
        }

        ctx.db.quest_chain_state().try_insert(QuestChainState{
            entity_id: game_state::create_entity(ctx),
            player_entity_id: actor_id,
            quest_chain_desc_id: desc.id,
            stage_id: desc.stages.first().copied().unwrap_or(0),
            completed: false,
            stage_rewards_awarded: Vec::new(),
            tracked: !desc.is_hint
        })?;
    }

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
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
    track_quest(ctx, actor_id, quest_chain_state)?;

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn set_quest_tracked(ctx: &ReducerContext, id : i32, tracked : bool) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut quest_chain_state = unwrap_or_err!(
        ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(&actor_id)
        .find(|qcs : &QuestChainState| qcs.quest_chain_desc_id == id),
        "Cannot track quest. Quest not started."
    );

    if !quest_chain_state.completed {
        if tracked {
            track_quest(ctx, actor_id, quest_chain_state)?;
        } else {
            quest_chain_state.tracked = false;
            ctx.db.quest_chain_state().entity_id().update(quest_chain_state);
        }
    }

    Ok(())
}

fn track_quest(ctx: &ReducerContext, player_entity_id: u64, mut quest_chain_state: QuestChainState) -> Result<(), String> {
    if !quest_chain_state.tracked {
        untrack_oldest_quest_if_needed(ctx, player_entity_id)?;
        quest_chain_state.tracked = true;
    }

    ctx.db.quest_chain_state().entity_id().update(quest_chain_state);
    Ok(())
}

fn untrack_oldest_quest_if_needed(ctx: &ReducerContext, player_entity_id: u64) -> Result<(), String> {
    const MAX_TRACKED_QUESTS: usize = 3;

    let tracked_quests: Vec<_> = ctx.db.quest_chain_state()
        .player_entity_id()
        .filter(player_entity_id)
        .filter(|quest_chain_state: &QuestChainState| quest_chain_state.tracked)
        .collect();

    if tracked_quests.len() >= MAX_TRACKED_QUESTS {
        let mut oldest_tracked_quest = unwrap_or_err!(
            tracked_quests.into_iter().min_by_key(|quest_chain_state| quest_chain_state.entity_id),
            "Cannot untrack the oldest quest."
        );
        oldest_tracked_quest.tracked = false;
        ctx.db.quest_chain_state().entity_id().update(oldest_tracked_quest);
    }

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
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
    let player_coord = ctx.db.mobile_entity_state().entity_id().find(&actor_id).unwrap().coordinates();
    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        actor_id,
        &onboarding_reward_desc.rewards,
        |x| x.distance_to(player_coord),
        false,
        || vec![player_coord],
        true,
    )?;

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
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

    quest_chain_state.tracked = false;
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
