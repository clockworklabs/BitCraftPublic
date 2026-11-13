use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state::{self},
    messages::global::{visibility_state, VisibilityState, VisibilityType},
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn set_visibility(ctx: &ReducerContext, visibility: VisibilityType) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let visibility_state = ctx.db.visibility_state().entity_id().find(actor_id);
    if let Some(mut visibility_state) = visibility_state {
        visibility_state.visibility = visibility;

        ctx.db.visibility_state().entity_id().update(visibility_state);

        return Ok(());
    }

    ctx.db.visibility_state().insert(VisibilityState {
        entity_id: actor_id,
        visibility,
    });

    Ok(())
}
