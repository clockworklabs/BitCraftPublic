use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::game::reducer_helpers::player_action_helpers;
use crate::{
    game::entities::building_state::InventoryState,
    messages::{action_request::PlayerItemUseRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_use_start(ctx: &ReducerContext, request: PlayerItemUseRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let delay = event_delay();
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::UseItem,
        None,
        None,
        delay,
        reduce(ctx, actor_id, request.pocket_index, request.arg_entity_id, true),
        game_state::unix_ms(ctx.timestamp),
    )
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_use(ctx: &ReducerContext, request: PlayerItemUseRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.pocket_index, request.arg_entity_id, false)
}

pub fn event_delay() -> Duration {
    Duration::from_secs(1)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, pocket_index: i32, _arg_entity_id: u64, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, entity_id, PlayerActionType::UseItem, None)?;
    }

    let mut player_inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, entity_id), "Invalid inventory");

    player_inventory.remove_quantity_at(pocket_index as usize, 1);
    ctx.db.inventory_state().entity_id().update(player_inventory);

    /*
    // Use Item
    let item_id = unwrap_or_err!(player_inventory.get_at(pocket_index as usize), "Missing item.").item_id;
    if let Some(teleport_item) = ctx.db.teleport_item_desc().id().find(&item_id) {
        if ThreatState::in_combat(ctx, entity_id) {
            return Err("Can't teleport while in combat!".into());
        }

        let vote = unwrap_or_err!(
            ctx.db.player_vote_state().entity_id().find(&arg_entity_id),
            "Query no longer exists."
        );

        if vote.initiator_entity_id != entity_id {
            return Err("You are not the instigator of this teleport request".into());
        }

        let target_player_entity_id = vote.participants_entity_id[1];

        // Verify target is still available
        if let Some(error) = teleport_to_player_request::can_be_teleported_to(ctx, entity_id, target_player_entity_id) {
            return Err(error.into());
        }

        let teleport_offset_cord = game_state_filters::offset_coordinates_float(ctx, target_player_entity_id);

        if dry_run {
            return Ok(());
        }

        // Grant player teleport item debuff
        if entities::buff::activate(ctx, entity_id, teleport_item.buff_id, None, None).is_err() {
            return Err("Unable to grant teleport item debuff to player".into());
        }

        // clear anyone targetting the dead player
        game_state_filters::untarget(ctx, entity_id);

        // remove deployable when UseIteming (from death or UseItem command)
        dismount_deployable(ctx, entity_id, false);

        // Teleport by server
        ctx.db
            .teleport_player_timer()
            .try_insert(TeleportPlayerTimer {
            scheduled_id: 0,
            scheduled_at: ctx.timestamp.into(),
            player_entity_id: entity_id,
            location: teleport_offset_cord,
            reason: crate::messages::action_request::ServerTeleportReason::TeleportItem,
        })
        .ok()
        .unwrap();
    }*/

    Ok(())
}
