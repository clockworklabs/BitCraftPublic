use crate::{
    game::handlers::cheats::cheat_type::{can_run_cheat, CheatType},
    mobile_entity_state, mounting_state,
};
use crate::messages::events::{player_teleport_event, PlayerTeleportEvent};
use spacetimedb::{ReducerContext, Table};

use crate::{messages::action_request::CheatTeleportFloatRequest, unwrap_or_err};

#[spacetimedb::reducer]
fn cheat_teleport_float(ctx: &ReducerContext, request: CheatTeleportFloatRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatTeleportFloat) {
        return Err("Unauthorized.".into());
    }

    let location = request.destination.unwrap();
    let mut mes = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(request.player_entity_id),
        "Invalid entity id"
    );
    mes.set_location(location);
    mes.set_destination(location);
    mes.dimension = 1;
    ctx.db.mobile_entity_state().entity_id().update(mes.clone());

    if ctx.db.mounting_state().entity_id().find(request.player_entity_id).is_some() {
        ctx.db.mobile_entity_state().entity_id().update(mes);
    }

    ctx.db.player_teleport_event().insert(PlayerTeleportEvent {
        actor_id: request.player_entity_id,
    });

    Ok(())
}
