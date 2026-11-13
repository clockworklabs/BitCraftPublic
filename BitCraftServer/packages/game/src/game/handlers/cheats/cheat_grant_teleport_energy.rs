use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::components::teleportation_energy_state;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_grant_teleport_energy(ctx: &ReducerContext, player_entity_id: u64, amount: f32) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatGrantTeleportEnergy) {
        return Err("Unauthorized.".into());
    }

    let mut ts = unwrap_or_err!(
        ctx.db.teleportation_energy_state().entity_id().find(player_entity_id),
        "Invalid player id"
    );
    ts.add_energy(ctx, amount);
    ctx.db.teleportation_energy_state().entity_id().update(ts);

    Ok(())
}
