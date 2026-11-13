use std::time::Duration;

use spacetimedb::{ReducerContext, Table, TimeDuration};

use crate::{
    alert_desc, alert_state,
    game::game_state,
    messages::{components::AlertState, static_data::AlertType},
};

impl AlertState {
    #[expect(clippy::new_ret_no_self, reason = "pre-existing signature, left as is when clippy was enabled")]
    pub fn new(ctx: &ReducerContext, alert_type: AlertType, player_entity_id: u64, target_entity_id: u64) -> Result<(), String> {
        let alert_desc = ctx.db.alert_desc().alert_type().find(alert_type as i32).unwrap();

        let mut new_alert = AlertState {
            entity_id: 0,
            player_entity_id,
            target_entity_id,
            alert_type,
            end_timestamp: ctx.timestamp + TimeDuration::from(Duration::from_secs_f32(alert_desc.duration * 60.0)),
        };

        // Can only have 1 alert of matching type, player and target
        if let Some(alert) = AlertState::get(ctx, alert_type, player_entity_id, target_entity_id) {
            new_alert.entity_id = alert.entity_id;
            ctx.db.alert_state().entity_id().update(new_alert);
        } else {
            new_alert.entity_id = game_state::create_entity(ctx);
            ctx.db.alert_state().try_insert(new_alert)?;
        }
        Ok(())
    }

    pub fn get(ctx: &ReducerContext, alert_type: AlertType, player_entity_id: u64, target_entity_id: u64) -> Option<AlertState> {
        let mut alerts = ctx
            .db
            .alert_state()
            .player_entity_id()
            .filter(player_entity_id)
            .filter(|a| a.alert_type == alert_type && a.target_entity_id == target_entity_id);
        alerts.next()
    }

    pub fn delete(ctx: &ReducerContext, alert_type: AlertType, player_entity_id: u64, target_entity_id: u64) {
        if let Some(alert) = AlertState::get(ctx, alert_type, player_entity_id, target_entity_id) {
            ctx.db.alert_state().entity_id().delete(alert.entity_id);
        }
    }

    pub fn on_sign_in(ctx: &ReducerContext, player_entity_id: u64) {
        let now = ctx.timestamp;
        // Clear all outdated alerts on signing in
        for alert in ctx.db.alert_state().player_entity_id().filter(player_entity_id) {
            let alert_desc = ctx.db.alert_desc().alert_type().find(alert.alert_type as i32).unwrap();
            if alert_desc.duration > 0.0 && alert.end_timestamp < now {
                ctx.db.alert_state().entity_id().delete(alert.entity_id);
            }
        }
    }
}
