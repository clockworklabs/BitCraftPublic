use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::components::{pillar_shaping_state, PillarShapingState};
use crate::{game::game_state, messages::action_request::PlayerPillarShapingPlaceRequest};
use spacetimedb::{ReducerContext, Table};

// Similar to pillar_shaping_add_pillar::reduce()
#[spacetimedb::reducer]
pub fn cheat_pillar_shaping_add_pillar(ctx: &ReducerContext, request: PlayerPillarShapingPlaceRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatPavingAddTile) {
        return Err("Unauthorized.".into());
    }

    let coordinates = request.coordinates.into();
    let pillar_type_id = request.pillar_type_id;

    // Delete existing pillar
    if let Some(existing_pillar_shaping) = PillarShapingState::get_at_location(ctx, &coordinates) {
        if existing_pillar_shaping.pillar_type_id == pillar_type_id {
            return Err("This pillar already has that type of decoration".into());
        }
        PillarShapingState::delete_pillar_shaping(ctx, existing_pillar_shaping.entity_id);
    }

    // Create pillar shaping tile
    let entity_id = game_state::create_entity(ctx);

    // location
    let target_coord = coordinates.center_small_tile();
    let offset = target_coord.to_offset_coordinates();
    game_state::insert_location(ctx, entity_id, offset);

    // pillar entity
    let pillar = PillarShapingState { entity_id, pillar_type_id };

    ctx.db.pillar_shaping_state().try_insert(pillar)?;

    Ok(())
}
