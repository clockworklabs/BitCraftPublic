use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    inter_module::transfer_player,
    messages::{components::user_previous_region_state, queue::player_queue_state},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_cancel_region_transfer(ctx: &ReducerContext) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, false)?;
    if ctx.db.player_queue_state().entity_id().find(actor_id).is_none() {
        return Err("Not in a queue".into());
    }
    let prev = unwrap_or_err!(
        ctx.db.user_previous_region_state().identity().find(ctx.sender()),
        "Not transfering regions"
    );
    if !prev.allow_cancel {
        return Err("Cannot cancel region transfer".into());
    }

    transfer_player::send_message(
        ctx,
        actor_id,
        prev.previous_region_location,
        prev.with_vehicle,
        -prev.teleport_energy_cost,
    )?;
    ctx.db.user_previous_region_state().identity().delete(ctx.sender());

    Ok(())
}
