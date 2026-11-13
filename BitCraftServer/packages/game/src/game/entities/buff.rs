use spacetimedb::{ReducerContext, Timestamp};

use crate::game::game_state::unix;
use crate::messages::game_util::ActiveBuff;
use crate::messages::static_data::{BuffCategory, BuffDesc};
use crate::{active_buff_state, unwrap_or_err};

impl ActiveBuff {
    pub fn has_cargo_debuff(ctx: &ReducerContext, player_entity_id: u64) -> bool {
        Self::has_active_buff_of_category(ctx, player_entity_id, BuffCategory::CarryCargo)
    }

    pub fn has_active_buff_of_category(ctx: &ReducerContext, player_entity_id: u64, buff_category: BuffCategory) -> bool {
        if buff_category.has_only_one_buff() {
            return Self::has_active_buff_of_type(ctx, player_entity_id, buff_category as i32);
        }
        if let Some(active_buff_state) = ctx.db.active_buff_state().entity_id().find(player_entity_id) {
            return active_buff_state.active_buff_of_category(ctx, buff_category).is_some();
        }
        false
    }

    pub fn has_active_buff_of_type(ctx: &ReducerContext, player_entity_id: u64, buff_type: i32) -> bool {
        if let Some(active_buff_state) = ctx.db.active_buff_state().entity_id().find(player_entity_id) {
            return active_buff_state.active_buff_of_type(ctx, buff_type).is_some();
        }
        false
    }

    pub fn expired(&self, now: Timestamp) -> bool {
        if self.buff_duration < 0 {
            return false;
        }
        let buff_end_time = self.buff_start_timestamp.value + self.buff_duration;
        let now = unix(now);
        now >= buff_end_time
    }
}

pub fn activate(ctx: &ReducerContext, entity_id: u64, buff_id: i32, duration: Option<i32>, values: Option<Vec<f32>>) -> Result<(), String> {
    let mut active_buff_state = unwrap_or_err!(
        ctx.db.active_buff_state().entity_id().find(entity_id),
        "Entity has no ActiveBuffState."
    );
    active_buff_state.add_active_buff_with_data(ctx, buff_id, duration, values);
    ctx.db.active_buff_state().entity_id().update(active_buff_state);

    Ok(())
}

pub fn deactivate(ctx: &ReducerContext, entity_id: u64, buff_id: i32) -> Result<(), String> {
    let mut active_buff_state = unwrap_or_err!(
        ctx.db.active_buff_state().entity_id().find(entity_id),
        "Entity has no ActiveBuffState."
    );
    active_buff_state.remove_active_buff(ctx, buff_id);
    ctx.db.active_buff_state().entity_id().update(active_buff_state);

    Ok(())
}

pub fn deactivate_sprint(ctx: &ReducerContext, entity_id: u64) {
    let sprint_buff_id = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::Sprint).unwrap().id;
    let _ = deactivate(ctx, entity_id, sprint_buff_id);
}
