use spacetimedb::{ReducerContext, Timestamp};

use crate::game::entities::player_state::INCAPACITATED_MESSAGE;
use crate::messages::components::{HealthState, PlayerState};
use crate::messages::static_data::*;
use crate::{building_state, enemy_state, health_state};

impl HealthState {
    pub fn check_incapacitated(ctx: &ReducerContext, entity_id: u64, notify_client: bool) -> Result<(), String> {
        let incapacitated;
        if let Some(health) = ctx.db.health_state().entity_id().find(entity_id) {
            incapacitated = health.health <= 0.0;
        } else {
            incapacitated = false;
        }

        if !incapacitated {
            return Ok(());
        }
        if notify_client {
            return Err(INCAPACITATED_MESSAGE.into());
        }
        Err("~Player is dead".into())
    }

    pub fn is_incapacitated_self(&self) -> bool {
        self.health <= 0.0
    }

    // also returns false if there is no healthstate
    pub fn is_alive(ctx: &ReducerContext, entity_id: u64) -> bool {
        if let Some(health) = ctx.db.health_state().entity_id().find(entity_id) {
            health.health > 0.0
        } else {
            false
        }
    }

    pub fn add_health_delta(&mut self, delta: f32, now: Timestamp) {
        if delta < 0.0 && self.health > 0.0 {
            self.last_health_decrease_timestamp = now;
        }
        self.health = (self.health + delta).max(0.0);
    }

    pub fn add_health_delta_clamped(&mut self, delta: f32, min: f32, max: f32, now: Timestamp) -> bool {
        if delta < 0.0 && self.health > min {
            self.last_health_decrease_timestamp = now;
        }

        let previous_health = self.health;
        self.health = (self.health + delta).clamp(min, max);

        self.health != previous_health
    }

    pub fn add_player_health(ctx: &ReducerContext, entity_id: u64, delta: f32) -> bool {
        if let Some(health) = ctx.db.health_state().entity_id().find(entity_id) {
            let mut health = health.clone();
            let previous = health.health;
            health.add_health_delta(delta, ctx.timestamp);
            health.health = health.health.min(HealthState::max_player_health(ctx, entity_id));
            if previous != health.health {
                ctx.db.health_state().entity_id().update(health);
            }
            return true;
        }
        false
    }

    pub fn update_player_health(ctx: &ReducerContext, mut health: HealthState, delta: f32) -> bool {
        let entity_id = health.entity_id;
        let previous = health.health;
        health.add_health_delta(delta, ctx.timestamp);
        health.health = health.health.min(HealthState::max_player_health(ctx, entity_id));
        if previous != health.health {
            ctx.db.health_state().entity_id().update(health);
            return true;
        }
        true
    }

    pub fn add_building_health(ctx: &ReducerContext, entity_id: u64, delta: f32) -> bool {
        if let Some(health) = ctx.db.health_state().entity_id().find(entity_id) {
            let mut health = health.clone();
            let previous = health.health;
            health.add_health_delta(delta, ctx.timestamp);
            health.health = health.health.min(HealthState::max_building_health(ctx, entity_id));
            if previous != health.health {
                ctx.db.health_state().entity_id().update(health);
            }
            return true;
        }
        false
    }

    pub fn add_enemy_health(ctx: &ReducerContext, entity_id: u64, delta: f32) -> bool {
        if let Some(health) = ctx.db.health_state().entity_id().find(entity_id) {
            let mut health = health.clone();
            let previous = health.health;
            health.add_health_delta(delta, ctx.timestamp);
            health.health = health.health.min(HealthState::max_enemy_health(ctx, entity_id));
            if previous != health.health {
                ctx.db.health_state().entity_id().update(health);
            }
            return true;
        }
        false
    }

    pub fn update_enemy_health(ctx: &ReducerContext, mut health: HealthState, delta: f32) -> bool {
        let entity_id = health.entity_id;
        let previous = health.health;
        health.add_health_delta(delta, ctx.timestamp);
        health.health = health.health.min(HealthState::max_enemy_health(ctx, entity_id));
        if previous != health.health {
            ctx.db.health_state().entity_id().update(health);
        }
        true
    }

    pub fn max_player_health(ctx: &ReducerContext, entity_id: u64) -> f32 {
        PlayerState::get_stat(ctx, entity_id, CharacterStatType::MaxHealth)
    }

    pub fn max_building_health(ctx: &ReducerContext, entity_id: u64) -> f32 {
        let building = ctx.db.building_state().entity_id().find(entity_id).unwrap();
        let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
        building_desc.max_health as f32
    }

    pub fn max_enemy_health(ctx: &ReducerContext, entity_id: u64) -> f32 {
        let enemy = ctx.db.enemy_state().entity_id().find(entity_id).unwrap();
        let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy.enemy_type as i32).unwrap();
        enemy_desc.max_health as f32
    }
}
