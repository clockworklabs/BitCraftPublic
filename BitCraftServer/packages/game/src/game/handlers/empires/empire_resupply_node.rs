use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::ReducerContext;

use crate::{
    game::{
        game_state::{self, game_state_filters},
        handlers::inventory::inventory_helper,
        reducer_helpers::player_action_helpers::{self, post_reducer_update_cargo},
    },
    inter_module::*,
    messages::{
        components::*,
        empire_shared::*,
        game_util::{ItemStack, ItemType, PocketKey},
        inter_module::*,
        static_data::{empire_supplies_desc, parameters_desc},
    },
    unwrap_or_err, unwrap_or_return,
};

use super::empires_shared::empire_resupply_node_validate;

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_resupply_node_start(ctx: &ReducerContext, request: EmpireResupplyNodeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let delay = ctx.db.parameters_desc().version().find(&0).unwrap().repair_building_duration as f32;
    let delay = Duration::from_secs_f32(delay);
    let target = request.building_entity_id;

    // DAB Note: TODO prevent CheatEngine and check timing for actions with duration

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::ResupplyEmpireNode,
        Some(target),
        None,
        delay,
        empire_resupply_node_reduce(ctx, actor_id, request.building_entity_id, request.from_pocket, true),
        game_state::unix_ms(ctx.timestamp),
    )
}

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_resupply_node(ctx: &ReducerContext, request: EmpireResupplyNodeRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::ResupplyEmpireNode.get_layer(ctx),
        empire_resupply_node_reduce(ctx, actor_id, request.building_entity_id, request.from_pocket, false),
    )
}

pub fn empire_resupply_node_reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    building_entity_id: u64,
    from_pocket: PocketKey,
    dry_run: bool,
) -> Result<(), String> {
    empire_resupply_node_validate(ctx, actor_id, building_entity_id)?;

    let player_location = game_state_filters::coordinates_any(ctx, actor_id);
    let watchtower_location = game_state_filters::coordinates(ctx, building_entity_id);
    if player_location.distance_to(watchtower_location) > 2 {
        return Err("Too far".into());
    }

    if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::SupplyNode) {
        return Err("You don't have the permissions to resupply a node".into());
    }

    // Check if the player still carries the cargo, if so, deposit it and active the node
    let mut inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(from_pocket.inventory_entity_id),
        "Missing inventory"
    );

    inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        inventory.owner_entity_id,
        inventory.player_owner_entity_id,
    )?;

    if from_pocket.pocket_index < 0 || from_pocket.pocket_index >= inventory.pockets.len() as i32 {
        return Err("Invalid pocket index".into());
    }
    let item_stack = unwrap_or_err!(inventory.get_at(from_pocket.pocket_index as usize), "Pocket is empty");
    if item_stack.item_type != ItemType::Cargo {
        return Err("Invalid item type".into());
    }

    let cargo_id = item_stack.item_id;

    let supplies = vec![ItemStack::single_cargo(cargo_id)];
    if !inventory.remove(&supplies) {
        return Err("The required supplies are no longer carried".into());
    }

    if dry_run {
        return Ok(());
    }

    ctx.db.inventory_state().entity_id().update(inventory);

    let supplies_count = ctx.db.empire_supplies_desc().cargo_id().find(&cargo_id).unwrap().energy;
    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireResupplyNode(EmpireResupplyNodeMsg {
            building_entity_id,
            supplies_count,
            player_entity_id: actor_id,
            cargo_id,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireResupplyNodeMsg, error: Option<String>) {
    if error.is_some() {
        //Add supply cargo if remote call fails
        let mut player_inventory = unwrap_or_return!(
            InventoryState::get_player_inventory(ctx, request.player_entity_id),
            "Player has no inventory"
        );
        let supplies = vec![ItemStack::single_cargo(request.cargo_id)];
        player_inventory.add_multiple_with_overflow(ctx, &supplies);
        ctx.db.inventory_state().entity_id().update(player_inventory);
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
