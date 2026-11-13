use spacetimedb::{log, ReducerContext, Table};

pub use crate::game::coordinates::*;
use crate::{
    game::game_state::{self, game_state_filters},
    messages::{authentication::ServerIdentity, components::*},
};

#[spacetimedb::table(accessor = duel_despawn_timer, scheduled(duel_despawn, at = scheduled_at))]
pub struct DuelDespawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
}

#[spacetimedb::reducer]
pub fn duel_despawn(ctx: &ReducerContext, timer: DuelDespawnTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to duel_despawn");
        return;
    }

    ctx.db.duel_state().entity_id().delete(timer.entity_id);
}

impl DuelState {
    pub fn initiate(ctx: &ReducerContext, initiator_entity_id: u64, acceptor_entity_id: u64) -> Result<(), String> {
        let location = game_state_filters::offset_coordinates_float(ctx, initiator_entity_id);
        let entity_id = game_state::create_entity(ctx);
        let player_entity_ids = vec![initiator_entity_id, acceptor_entity_id];

        let duel = DuelState {
            victor: None,
            entity_id,
            initiator_entity_id,
            acceptor_entity_id,
            player_entity_ids,
            out_of_range_timestamps: vec![None, None],
        };

        ctx.db.duel_state().try_insert(duel)?;
        game_state::insert_location_float(ctx, entity_id, location);

        Ok(())
    }

    pub fn update_out_of_range_timestamp(&mut self, ctx: &ReducerContext, player_index: usize, out_of_range: bool) -> bool {
        if out_of_range {
            if self.out_of_range_timestamps[player_index].is_none() {
                self.out_of_range_timestamps[player_index] = Some(ctx.timestamp);
                return true;
            }
        } else {
            if self.out_of_range_timestamps[player_index].is_some() {
                self.out_of_range_timestamps[player_index] = None;
                return true;
            }
        }
        false
    }

    pub fn timed_out(&self, ctx: &ReducerContext, player_index: usize, timeout_millis: i32) -> bool {
        if let Some(timestamp) = self.out_of_range_timestamps[player_index] {
            return (ctx.timestamp.duration_since(timestamp).unwrap().as_millis() as i32) >= timeout_millis;
        }
        false
    }

    pub fn set_loser(&mut self, ctx: &ReducerContext, player_index: usize) {
        // schedule self-delete

        self.victor = Some(self.player_entity_ids[1 - player_index]);

        ctx.db.duel_despawn_timer().insert(DuelDespawnTimer {
            scheduled_id: 0,
            scheduled_at: ctx.timestamp.into(),
            entity_id: self.entity_id,
        });
    }

    pub fn are_players_dueling(ctx: &ReducerContext, attacker_entity_id: u64, defender_entity_id: u64) -> bool {
        if let Some(duel) = Self::get_for_player(ctx, attacker_entity_id) {
            return duel.player_entity_ids.contains(&defender_entity_id);
        }
        false
    }

    pub fn get_for_player(ctx: &ReducerContext, player_entity_id: u64) -> Option<Self> {
        if let Some(duel) = ctx.db.duel_state().initiator_entity_id().find(player_entity_id) {
            return Some(duel);
        }
        ctx.db.duel_state().acceptor_entity_id().find(player_entity_id)
    }
}
