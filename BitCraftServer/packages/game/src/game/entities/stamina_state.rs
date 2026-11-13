use crate::messages::components::{PlayerState, StaminaState};
use crate::messages::static_data::*;
use crate::stamina_state;
use spacetimedb::{ReducerContext, Timestamp};

impl StaminaState {
    pub fn set_stamina(&mut self, stamina: f32, now: Timestamp) {
        if stamina < self.stamina {
            self.last_stamina_decrease_timestamp = now;
        }
        self.stamina = stamina
    }

    fn add_stamina_delta(&mut self, delta: f32, now: Timestamp) {
        if (delta < 0.0) & (self.stamina > 0.0) {
            self.last_stamina_decrease_timestamp = now;
        }
        self.stamina = (self.stamina + delta).max(0.0);
    }

    pub fn add_stamina_delta_clamped(&mut self, delta: f32, min: f32, max: f32, now: Timestamp) -> bool {
        if (delta < 0.0) & (self.stamina > min) {
            self.last_stamina_decrease_timestamp = now;
        }

        let previous_stamina = self.stamina;
        self.stamina = (self.stamina + delta).clamp(min, max);
        self.stamina != previous_stamina
    }

    pub fn add_player_stamina(ctx: &ReducerContext, entity_id: u64, delta: f32) -> bool {
        if delta == 0.0 {
            return true;
        }

        if let Some(mut stamina) = ctx.db.stamina_state().entity_id().find(entity_id) {
            stamina.add_stamina_delta(delta, ctx.timestamp);
            if delta > 0.0 {
                stamina.stamina = stamina.stamina.min(StaminaState::max_player_stamina(ctx, entity_id));
            }
            ctx.db.stamina_state().entity_id().update(stamina);
            return true;
        }
        false
    }

    pub fn set_player_stamina(ctx: &ReducerContext, entity_id: u64, new_stamina: f32) -> bool {
        if let Some(mut stamina) = ctx.db.stamina_state().entity_id().find(entity_id) {
            stamina.set_stamina(new_stamina, ctx.timestamp);
            ctx.db.stamina_state().entity_id().update(stamina);
            return true;
        }
        false
    }

    pub fn decrease_stamina(ctx: &ReducerContext, entity_id: u64, value: f32) -> bool {
        if let Some(stamina) = ctx.db.stamina_state().entity_id().find(entity_id) {
            if stamina.stamina >= value {
                let mut stamina = stamina.clone();
                stamina.add_stamina_delta(-value.abs(), ctx.timestamp);
                ctx.db.stamina_state().entity_id().update(stamina);
                return true;
            }
        }
        false
    }

    pub fn max_player_stamina(ctx: &ReducerContext, entity_id: u64) -> f32 {
        PlayerState::get_stat(ctx, entity_id, CharacterStatType::MaxStamina)
    }
}
