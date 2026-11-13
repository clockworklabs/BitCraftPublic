use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::global::player_shard_state;
use spacetimedb::ReducerContext;

use crate::messages::action_request::CheatShardsGrantRequest;
use crate::unwrap_or_err;

#[spacetimedb::reducer]
pub fn cheat_shards_grant(ctx: &ReducerContext, request: CheatShardsGrantRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatShardsGrant) {
        return Err("Unauthorized.".into());
    }

    let mut vault_state = unwrap_or_err!(
        ctx.db.player_shard_state().entity_id().find(request.owner_entity_id),
        "Player does not exist"
    );
    vault_state.shards += request.shards;
    ctx.db.player_shard_state().entity_id().update(vault_state);
    Ok(())
}
