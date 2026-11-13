use bitcraft_macro::feature_gate;
use crate::{
    game::game_state,
    messages::{components::deployable_collectible_state, static_data::deployable_desc},
    unwrap_or_err,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_toggle_auto_follow(ctx: &ReducerContext, deployable_desc_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let deployable_desc = unwrap_or_err!(ctx.db.deployable_desc().id().find(deployable_desc_id), "Unknown deployable");
    if !deployable_desc.can_auto_follow {
        return Err("This deployable can't auto-follow".into());
    }

    let mut deployable_collectible_state = unwrap_or_err!(
        ctx.db
            .deployable_collectible_state()
            .owner_entity_id()
            .filter(actor_id)
            .find(|c| c.deployable_desc_id == deployable_desc_id),
        "You do not own a deployable of that type"
    );

    deployable_collectible_state.auto_follow = !deployable_collectible_state.auto_follow;

    ctx.db
        .deployable_collectible_state()
        .deployable_entity_id()
        .update(deployable_collectible_state);

    Ok(())
}
