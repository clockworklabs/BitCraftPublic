use crate::game::game_state::game_state_filters;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{game::coordinates::*, messages::action_request::CheatWarpRequest};
use crate::messages::events::{player_teleport_event, PlayerTeleportEvent};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn cheat_warp(ctx: &ReducerContext, request: CheatWarpRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatWarp) {
        return Err("Unauthorized.".into());
    }

    let oc_large = request.location;
    let hc_large = LargeHexTile::from(oc_large);
    let hc_small = SmallHexTile::from(hc_large);
    reduce(ctx, request.owner_entity_id, OffsetCoordinatesSmall::from(hc_small))?;
    ctx.db.player_teleport_event().insert(PlayerTeleportEvent {
        actor_id: request.owner_entity_id,
    });
    Ok(())
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, coordinates: OffsetCoordinatesSmall) -> Result<(), String> {
    let ofc: OffsetCoordinatesFloat = coordinates.into();
    game_state_filters::teleport_to(ctx, entity_id, ofc, true, 0.0)
}
