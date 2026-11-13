use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::messages::components::*;
use crate::{onboarding_reward_desc, unwrap_or_err};

#[spacetimedb::reducer]
#[feature_gate]
pub fn complete_onboarding_state(ctx: &ReducerContext, id: u16) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut onboarding_state = unwrap_or_err!(
        ctx.db.onboarding_state().entity_id().find(actor_id),
        "Failed to get onboarding state!"
    );

    if !onboarding_state.completed_states.contains(&id) {
        onboarding_state.completed_states.push(id);

        if let Some(reward) = ctx.db.onboarding_reward_desc().state_id().find(id) {
            let mut discovery = Discovery::new(actor_id);
            let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

            for mut item_stack in reward.item_stack_rewards {
                discovery.acquire_item_stack(ctx, &item_stack);
                item_stack.auto_collect(ctx, &mut discovery, actor_id);
                inventory.add_multiple_with_overflow(ctx, &vec![item_stack]);
            }

            ctx.db.inventory_state().entity_id().update(inventory);
            discovery.commit(ctx);
        }

        ctx.db.onboarding_state().entity_id().update(onboarding_state);
    }

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn start_onboarding_quest(ctx: &ReducerContext, id: u16) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut onboarding_state = unwrap_or_err!(
        ctx.db.onboarding_state().entity_id().find(actor_id),
        "Failed to get onboarding state!"
    );

    onboarding_state.current_quests.push(id);

    ctx.db.onboarding_state().entity_id().update(onboarding_state);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn complete_onboarding_quest(ctx: &ReducerContext, id: u16) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut onboarding_state = unwrap_or_err!(
        ctx.db.onboarding_state().entity_id().find(actor_id),
        "Failed to get onboarding state!"
    );

    onboarding_state.current_quests.retain(|x| *x != id);
    onboarding_state.completed_quests.push(id);

    ctx.db.onboarding_state().entity_id().update(onboarding_state);

    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn reset_onboarding(ctx: &ReducerContext) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let new_state = OnboardingState {
        entity_id: actor_id,
        completed_states: vec![],
        current_quests: vec![],
        completed_quests: vec![],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}
