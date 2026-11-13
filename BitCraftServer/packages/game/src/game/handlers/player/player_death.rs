use crate::game::game_state;
use crate::game::game_state::game_state_filters;
use crate::game::reducer_helpers::cargo_helpers::spawn_cargo;
use crate::game::reducer_helpers::player_action_helpers::post_reducer_update_cargo;
use crate::game::reducer_helpers::{deployable_helpers, player_action_helpers};
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::PlayerActionType;
use crate::messages::events::{player_death_event, player_death_start_event, PlayerDeathEvent, PlayerDeathStartEvent};
use crate::{inventory_state, parameters_desc, unwrap_or_err, InventoryState, ThreatState};
use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

#[spacetimedb::table(accessor = player_death_timer, scheduled(player_death_start, at = scheduled_at))]
pub struct PlayerDeathTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    #[unique]
    pub player_entity_id: u64,
}

#[spacetimedb::reducer]
#[feature_gate]
fn player_death_start(ctx: &ReducerContext, timer: PlayerDeathTimer) -> Result<(), String> {
    ServerIdentity::validate_server_or_admin(ctx)?;

    let player_entity_id = timer.player_entity_id;

    let respawn_seconds = ctx.db.parameters_desc().version().find(0).unwrap().respawn_seconds;
    let delay = Duration::from_secs(respawn_seconds as u64);

    deployable_helpers::dismount_deployable(ctx, player_entity_id, false);

    // Death is the final frontier. It can't fail.
    player_action_helpers::start_action(
        ctx,
        player_entity_id,
        PlayerActionType::Death,
        None,
        None,
        delay,
        Ok(()),
        game_state::unix_ms(ctx.timestamp),
    )
    .unwrap();
    ctx.db.player_death_start_event().insert(PlayerDeathStartEvent {
        actor_id: player_entity_id,
        timer,
    });

    // IMPORTANT:
    // Don't remove the player from the quad - we want to see the enemies continue moving around while dead.
    // The respawn teleport will update the player quad if needed

    // end all combat sessions involving this player
    ThreatState::clear_all(ctx, player_entity_id);

    // clear anyone targetting the dead player
    game_state_filters::untarget(ctx, player_entity_id);

    //Make player drop cargo
    let mut inventory = unwrap_or_err!(
        InventoryState::get_player_inventory(ctx, player_entity_id),
        "Player has no inventory"
    );
    if let Some(cargo) = inventory.get_pocket_contents(inventory.cargo_index as usize) {
        inventory.set_at(inventory.cargo_index as usize, None);
        ctx.db.inventory_state().entity_id().update(inventory);

        let coord = game_state_filters::coordinates_float(ctx, player_entity_id).parent_small_tile();
        spawn_cargo(ctx, player_entity_id, coord, cargo.item_id, 1);
    }

    post_reducer_update_cargo(ctx, player_entity_id);
    ctx.db.player_death_event().insert(PlayerDeathEvent { player_entity_id });

    Ok(())
}
