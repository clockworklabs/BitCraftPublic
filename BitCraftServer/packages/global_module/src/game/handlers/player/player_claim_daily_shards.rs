use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{game::game_state, messages::global::player_shard_state, parameters_desc, unwrap_or_err};

const SECONDS_IN_A_DAY: i32 = 24 * 60 * 60;

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_claim_daily_shards(ctx: &ReducerContext) -> Result<(), String> {
    let daily_shards = ctx.db.parameters_desc().version().find(0).unwrap().daily_shards;
    if daily_shards <= 0 {
        return Err("Claiming daily shards is disabled".into());
    }

    let actor_id = game_state::actor_id(ctx, true)?;
    let day = game_state::unix(ctx.timestamp) / SECONDS_IN_A_DAY;

    let mut vault = unwrap_or_err!(ctx.db.player_shard_state().entity_id().find(actor_id), "Player has no vault state");
    if vault.last_shard_claim >= day {
        return Err("You already claimed your shards today.".into());
    }

    vault.shards += daily_shards as u32;
    vault.last_shard_claim = day;
    ctx.db.player_shard_state().entity_id().update(vault);

    Ok(())
}
