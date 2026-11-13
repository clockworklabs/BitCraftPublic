use bitcraft_macro::feature_gate;
use crate::game::game_state::{self, game_state_filters};
use crate::messages::components::{active_buff_state, HealthState, PlayerActionState, StaminaState};
use crate::messages::events::{player_teleport_event, PlayerTeleportEvent};
use crate::{health_state, parameters_desc, unwrap_or_err, SatiationState};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use super::sleep;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
fn player_respawn(ctx: &ReducerContext, teleport_home: bool) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let respawn_seconds = ctx.db.parameters_desc().version().find(0).unwrap().respawn_seconds;

    // Revive with 1 hp
    let mut health_state = unwrap_or_err!(ctx.db.health_state().entity_id().find(actor_id), "Unable to update actor health");

    if health_state.health > 0.0 {
        return Err("You cannot respawn while still alive".into());
    }

    if health_state.died_timestamp + respawn_seconds > game_state::unix(ctx.timestamp) {
        return Err("You are not able to respawn yet".into());
    }

    health_state.health = if teleport_home {
        HealthState::max_player_health(ctx, actor_id)
    } else {
        10.0
    };
    ctx.db.health_state().entity_id().update(health_state);

    if !StaminaState::set_player_stamina(ctx, actor_id, 1.0) {
        return Err("No such player.".into());
    }

    SatiationState::add_player_satiation(ctx, actor_id, f32::MAX); //Method will clamp internally

    let mut active_buff_state = unwrap_or_err!(
        ctx.db.active_buff_state().entity_id().find(actor_id),
        "Player has no active buff state."
    );
    let innerlight_buff_duration = ctx.db.parameters_desc().version().find(0).unwrap().respawn_aggro_immunity;
    active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);
    ctx.db.active_buff_state().entity_id().update(active_buff_state);

    if teleport_home {
        game_state_filters::teleport_home(ctx, actor_id, true)?;

        // Sleep if there's a sleep building on the end-point. Ignore the error if there's no building.
        let _ = sleep::reduce(ctx, actor_id);
        ctx.db.player_teleport_event().insert(PlayerTeleportEvent { actor_id });
    } else {
        let _ = PlayerActionState::clear_by_entity_id(ctx, actor_id);
    }

    Ok(())
}
