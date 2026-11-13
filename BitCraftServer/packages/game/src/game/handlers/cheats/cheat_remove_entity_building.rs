use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::reducer_helpers::building_helpers::delete_building;
use crate::game::{dimensions, game_state::game_state_filters};
use crate::portal_state;

use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn cheat_remove_entity_building(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatRemoveEntityBuilding) {
        return Err("Unauthorized.".into());
    }

    let coordinates = game_state_filters::coordinates(ctx, building_entity_id);

    if coordinates.dimension != dimensions::OVERWORLD
        && ctx.db.portal_state().entity_id().find(building_entity_id).is_some() {
            // NOTE: We are preventing deleteing portals as this could leave interiors in a weird state.
            // In the future there may be portal buildings inside interiors that lead to *nested* interiors.
            // This may be a case where we want to allow deleting portals, but currently isn't allowed.
            return Err("Can't deconstruct portal buildings inside interiors".into());
        }

    delete_building(ctx, 0, building_entity_id, None, false, false);

    Ok(())
}
