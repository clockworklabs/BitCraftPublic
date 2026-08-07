use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    messages::{
        action_request::PlayerCompleteTaskRequest,
        components::*,
        static_data::{npc_desc, traveler_task_desc, traveler_task_knowledge_requirement_desc},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_complete_task(ctx: &ReducerContext, request: PlayerCompleteTaskRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut task = unwrap_or_err!(
        ctx.db.traveler_task_state().entity_id().find(request.task_entity_id),
        "This task no longer exists"
    );

    if task.player_entity_id != actor_id {
        return Err("This task is not yours to turn in".into());
    }

    if task.completed {
        return Err("This task has already been turned in".into());
    }

    let task_desc = unwrap_or_err!(
        ctx.db.traveler_task_desc().id().find(task.task_id),
        "This is an unknown type of task"
    );

    let npc = unwrap_or_err!(
        ctx.db.npc_state().entity_id().find(request.npc_entity_id),
        "This traveler doesn't exist"
    );

    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);
    let npc_coord = game_state_filters::coordinates_any(ctx, request.npc_entity_id);
    if npc_coord.distance_to(player_coord) > 8 {
        return Err("Too far".into());
    }

    if npc.npc_type as i32 != task.traveler_id {
        return Err("This traveler did not issue this task".into());
    }

    let npc_desc = ctx.db.npc_desc().npc_type().find(npc.npc_type as i32).unwrap();

    if !npc_desc.task_skill_check.contains(&task_desc.level_requirement.skill_id) {
        return Err("This traveler does not offer that kind of task".into());
    }

    let player_knowledges = ctx.db.knowledge_secondary_state().entity_id().find(actor_id).unwrap();
    if let Some(task_requirements) = ctx
        .db
        .traveler_task_knowledge_requirement_desc()
        .traveler_task_id()
        .find(task.task_id)
    {
        if !task_requirements.required_knowledges.is_empty()
            && !task_requirements
                .required_knowledges
                .iter()
                .all(|knowledge_id| player_knowledges.is_acquired(*knowledge_id))
        {
            return Err("You don't have the knowledge required to complete this task".into());
        }
        // Blocking knowledges are used to prevent tasks from being offered, and are not checked here
    }

    // NOT adding blocking knowledges to turn in the task, in case the knowledge was acquired AFTER delivering the task

    let mut inventory = InventoryState::get_player_inventory(ctx, actor_id).unwrap();
    if !inventory.remove(&task_desc.required_items) {
        return Err("You do not have the required items".into());
    }
    ctx.db.inventory_state().entity_id().update(inventory);

    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        actor_id,
        &task_desc.rewarded_items,
        |x| npc_coord.distance_to(x),
        true,
        || vec![{ game_state_filters::coordinates_any(ctx, actor_id) }],
        false,
    )?;

    ExperienceState::add_experience(
        ctx,
        actor_id,
        task_desc.rewarded_experience.skill_id,
        task_desc.rewarded_experience.quantity as i32,
    );

    task.completed = true;
    ctx.db.traveler_task_state().entity_id().update(task.clone());

    let requests = TravelerTaskState::generate_npc_requests_hashmap(ctx);
    TravelerTaskState::replace_completed_task(
        ctx,
        actor_id,
        task.entity_id,
        task.traveler_id,
        task.task_id,
        1,
        &requests,
    );

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
