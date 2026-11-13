use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::{ReducerContext, Table};

use crate::game::claim_helper;
use crate::game::handlers::inventory::inventory_helper;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::ClaimResupplyRequest;
use crate::messages::events::*;
use crate::messages::game_util::ItemType;
use crate::{building_repairs_desc, parameters_desc};
use crate::{
    game::{
        game_state::{self, game_state_filters},
        permission_helper,
    },
    messages::components::*,
    unwrap_or_err,
};

pub fn event_delay(ctx: &ReducerContext) -> Duration {
    let delay = ctx.db.parameters_desc().version().find(0).unwrap().repair_building_duration as f32;

    Duration::from_secs_f32(delay)
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_resupply_start(ctx: &ReducerContext, request: ClaimResupplyRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let target = Some(request.building_entity_id);
    let delay = event_delay(ctx);
    let result = player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::ResupplyClaim,
        target,
        None,
        delay,
        reduce(ctx, actor_id, game_state::unix_ms(ctx.timestamp), &request, true),
        game_state::unix_ms(ctx.timestamp),
    );
    if result.is_ok() {
        ctx.db
            .claim_resupply_start_event()
            .insert(ClaimResupplyStartEvent { actor_id, request });
    }
    result
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_resupply(ctx: &ReducerContext, request: ClaimResupplyRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::ResupplyClaim.get_layer(ctx),
        reduce(ctx, actor_id, game_state::unix_ms(ctx.timestamp), &request, false),
    )
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, _timestamp: u64, request: &ClaimResupplyRequest, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::ResupplyClaim, Some(request.building_entity_id))?;
    }
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building_entity_id = request.building_entity_id;
    let building_state = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(building_entity_id),
        "No such building to repair."
    );

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Usage) {
        return Err("You don't have permission to interact with this building".into());
    }

    if !permission_helper::can_interact_with_building(ctx, &building_state, actor_id, ClaimPermission::Usage) {
        return Err("You don't have permission to interact with this building".into());
    }

    let building_coord = game_state_filters::coordinates_any(ctx, building_entity_id);
    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);

    if building_coord.distance_to(player_coord) > 3 {
        return Err("Too far".into());
    }

    let claim_tile = claim_helper::get_claim_on_tile(ctx, building_coord);

    if claim_tile.is_none() {
        return Err("Building is not under a claim".into());
    }

    let claim_desc_id = match claim_tile {
        Some(t) => t.claim_id,
        None => 0,
    };

    let claim_desc = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_desc_id), "Claim doesn't exist");
    let claim_local = claim_desc.local_state(ctx);
    let max_supplies = ctx
        .db
        .claim_tech_state()
        .entity_id()
        .find(claim_desc_id)
        .unwrap()
        .max_supplies(ctx);
    let supplies = claim_local.supplies as f32;

    if supplies >= max_supplies {
        return Err("This claim is fully supplied.".into());
    }
    if claim_desc.owner_building_entity_id != building_entity_id {
        return Err("You can't supply this building".into());
    }

    if supplies >= max_supplies {
        return Err("Claim is already fully supplied.".into());
    }

    // find stack with a repair kit
    let mut inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(request.from_pocket.inventory_entity_id),
        "Invalid inventory"
    );

    if request.from_pocket.pocket_index < 0 || request.from_pocket.pocket_index >= inventory.pockets.len() as i32 {
        return Err("Invalid pocket".into());
    }

    // Make sure player can currently interact with specified inventory
    inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_coord,
        inventory.owner_entity_id,
        inventory.player_owner_entity_id,
    )?;

    let stack = unwrap_or_err!(
        inventory.get_at(request.from_pocket.pocket_index as usize),
        "Inventory pocket is empty"
    );
    if stack.item_type != ItemType::Cargo {
        return Err("Claims can only be charged with supplies.".into());
    }
    let repair_value = match ctx.db.building_repairs_desc().cargo_id().find(stack.item_id) {
        Some(rep) => rep.repair_value,
        None => return Err("Claims can only be charged with supplies.".into()),
    };
    let max_quantity = ((max_supplies - supplies) as i32 + repair_value - 1) / repair_value;
    let quantity = stack.quantity.clamp(1, max_quantity);
    inventory
        .remove_quantity_at(request.from_pocket.pocket_index as usize, quantity)
        .unwrap();
    let repair_value = ((repair_value * quantity) as f32).min(max_supplies - supplies);

    if !dry_run {
        ctx.db.inventory_state().entity_id().update(inventory);

        // From a repair kit: recharge shield or repair building
        let _ = claim_local.update_supplies_and_commit(ctx, repair_value, false);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
