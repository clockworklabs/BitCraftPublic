use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::components::{progressive_action_state, public_progressive_action_state, HealthState, PublicProgressiveActionState},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn craft_set_public(ctx: &ReducerContext, progressive_action_entity_id: u64, is_public: bool) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    let progressive_action_state = unwrap_or_err!(
        ctx.db.progressive_action_state().entity_id().find(progressive_action_entity_id),
        "Craft not found"
    );

    if progressive_action_state.owner_entity_id != actor_id {
        return Err("You don't own this craft".into());
    }

    if is_public {
        if ctx
            .db
            .public_progressive_action_state()
            .try_insert(PublicProgressiveActionState {
                entity_id: progressive_action_state.entity_id,
                building_entity_id: progressive_action_state.building_entity_id,
                owner_entity_id: progressive_action_state.owner_entity_id,
            })
            .is_err()
        {
            return Err("Failed to make craft public".into());
        }

        return Ok(());
    }

    ctx.db
        .public_progressive_action_state()
        .entity_id()
        .delete(progressive_action_entity_id);

    Ok(())
}
