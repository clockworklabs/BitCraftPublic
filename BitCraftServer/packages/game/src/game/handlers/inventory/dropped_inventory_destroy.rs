use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self, game_state_filters},
    messages::components::{dropped_inventory_state, HealthState, PlayerTimestampState},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn dropped_inventory_destroy(ctx: &ReducerContext, dropped_inventory_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let dropped_inventory = unwrap_or_err!(
        ctx.db.dropped_inventory_state().entity_id().find(dropped_inventory_entity_id),
        "Dropped inventory is no longer available"
    );

    let player_coordinates_float = game_state_filters::coordinates_float(ctx, actor_id);
    let pile_coordinates = game_state_filters::coordinates(ctx, dropped_inventory_entity_id);

    if pile_coordinates.distance_to(player_coordinates_float.into()) > 3 {
        return Err("Too far".into());
    }

    if dropped_inventory.owner_entity_id != 0 {
        return Err("This dropped inventory is not public".into());
    }

    dropped_inventory.delete(ctx);

    Ok(())
}
