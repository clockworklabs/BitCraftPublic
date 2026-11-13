use crate::{player_timestamp_state, PlayerTimestampState};
use spacetimedb::{ReducerContext, Table, Timestamp};

impl PlayerTimestampState {
    pub fn refresh(ctx: &ReducerContext, actor_id: u64, timestamp: Timestamp) {
        if let Some(mut entry) = ctx.db.player_timestamp_state().entity_id().find(actor_id) {
            entry.timestamp = timestamp;
            ctx.db.player_timestamp_state().entity_id().update(entry);
        } else {
            let _ = ctx.db.player_timestamp_state().try_insert(PlayerTimestampState {
                entity_id: actor_id,
                timestamp,
            });
        }
    }

    pub fn clear(ctx: &ReducerContext, actor_id: u64) {
        ctx.db.player_timestamp_state().entity_id().delete(actor_id);
    }
}
