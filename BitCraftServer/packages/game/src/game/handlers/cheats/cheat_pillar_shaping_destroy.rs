use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::action_request::PlayerPillarShapingDestroyRequest;
use crate::messages::components::PillarShapingState;
use spacetimedb::ReducerContext;

// Similar to pillar_shaping_destroy::reduce()
#[spacetimedb::reducer]
pub fn cheat_pillar_shaping_destroy(ctx: &ReducerContext, request: PlayerPillarShapingDestroyRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatPillarShapingDestroy) {
        return Err("Unauthorized.".into());
    }

    let coordinates = request.coordinates.into();

    // Delete existing pillar
    if let Some(existing_pillar_shaping) = PillarShapingState::get_at_location(ctx, &coordinates) {
        PillarShapingState::delete_pillar_shaping(ctx, existing_pillar_shaping.entity_id);
    }

    Ok(())
}
