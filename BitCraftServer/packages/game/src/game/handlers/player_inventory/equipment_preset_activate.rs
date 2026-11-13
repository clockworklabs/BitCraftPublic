use crate::game::game_state::{self};
use crate::messages::components::*;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn equipment_preset_activate(ctx: &ReducerContext, index: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let mut recalculate_stats = false;

    for mut preset in ctx.db.equipment_preset_state().player_entity_id().filter(actor_id) {
        let activate = index == preset.index;
        if preset.active != activate {
            recalculate_stats = true;
            preset.active = activate;
            ctx.db.equipment_preset_state().entity_id().update(preset);
        }
    }

    if recalculate_stats {
        PlayerState::collect_stats(ctx, actor_id);
    }
    Ok(())
}
