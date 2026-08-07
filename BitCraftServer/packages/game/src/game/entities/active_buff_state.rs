use std::collections::HashMap;

use spacetimedb::{ReducerContext, Table, Timestamp};

use crate::game::game_state::unix;
use crate::game::reducer_helpers::stats_helpers::{collect_stats_timer, CollectStatsTimer};
use crate::game::reducer_helpers::timer_helpers::now_plus_secs_f32;
use crate::messages::components::{active_buff_state, player_state, ActiveBuffState, PlayerState};
use crate::messages::game_util::{ActiveBuff, OnlineTimestamp};
use crate::messages::static_data::*;

//
// Buffs Handling
//

impl ActiveBuffState {
    fn buff_index_or_add(&mut self, buff_id: i32) -> usize {
        let mut index = self.active_buffs.iter().position(|b| b.buff_id == buff_id);
        // Add a default instance of a buff if the buff id is not found
        if index.is_none() {
            self.active_buffs.push(ActiveBuff {
                buff_id,
                buff_start_timestamp: OnlineTimestamp { value: 0 },
                buff_duration: 0,
                values: Vec::new(),
            });
            index = Some(self.active_buffs.len() - 1);
        }
        index.unwrap()
    }

    fn buff_index(&self, buff_id: i32) -> Option<usize> {
        self.active_buffs.iter().position(|b| b.buff_id == buff_id)
    }

    // Activates a buff of the given id. Fills the buffstate with the default recipe data.
    pub fn add_active_buff(&mut self, ctx: &ReducerContext, buff_id: i32) {
        self.add_active_buff_with_data(ctx, buff_id, None, None);
    }

    // Alternative way to start an active buff providing possible overrides for duration, param1 and param2 (None to leave the default)
    pub fn add_active_buff_with_data(&mut self, ctx: &ReducerContext, buff_id: i32, duration: Option<i32>, values: Option<Vec<f32>>) {
        let buff_description = ctx.db.buff_desc().id().find(&buff_id).unwrap();

        let index = self.buff_index_or_add(buff_id);

        let mut buff_id_to_remove = 0; // mut/immutable borrow solver

        let new_duration = match duration {
            Some(d) => d,
            None => buff_description.duration,
        };

        // get the recipe default parameter values
        let new_values = match values {
            Some(v) => v,
            None => (&buff_description.stats).into_iter().map(|s| s.value).collect(),
        };

        if let Some(concurrent_buff) = self.active_buff_of_type(ctx, buff_description.buff_type_id) {
            let concurrent_desc = ctx.db.buff_desc().id().find(&concurrent_buff.buff_id).unwrap();
            if buff_description.priority < concurrent_desc.priority {
                // previous buff has higher priority, so this buff won't be added
                return;
            }
            // new buff is as or more potent, it will replace the previous one
            buff_id_to_remove = concurrent_buff.buff_id;
        }

        if buff_id_to_remove != 0 {
            self.remove_active_buff_internal(buff_id_to_remove);
        }

        let buff_state = self.active_buffs.get_mut(index).unwrap();

        buff_state.buff_start_timestamp = OnlineTimestamp {
            value: unix(ctx.timestamp),
        };

        buff_state.buff_duration = new_duration;
        buff_state.values = new_values;

        // Players will update stats and recalculate values at buff expiration.
        // Enemies don't need that - they simply apply buffs to their basic stats during attack or movement
        if ctx.db.player_state().entity_id().find(self.entity_id).is_some() {
            PlayerState::collect_stats_with_uncommited_buffs(ctx, &self);

            // schedule buff stats removal for when it expires
            if new_duration >= 0 {
                ctx.db
                    .collect_stats_timer()
                    .try_insert(CollectStatsTimer {
                        scheduled_id: 0,
                        scheduled_at: now_plus_secs_f32(new_duration as f32 + 0.5, ctx.timestamp),
                        entity_id: self.entity_id,
                    })
                    .ok()
                    .unwrap();
            }
        }
    }

    // Sets the timestamp and duration of the buff index to 0, effectively making it non-persistent and expired.
    pub fn remove_active_buff(&mut self, ctx: &ReducerContext, buff_id: i32) {
        self.remove_active_buff_internal(buff_id);
        PlayerState::collect_stats_with_uncommited_buffs(ctx, &self);
    }

    fn remove_active_buff_internal(&mut self, buff_id: i32) {
        let index = self.buff_index_or_add(buff_id);
        let buff_state = self.active_buffs.get_mut(index).unwrap();
        buff_state.buff_start_timestamp = OnlineTimestamp { value: 0 };
        buff_state.buff_duration = 0;
    }

    pub fn remove_all_active_buffs(&mut self, ctx: &ReducerContext) {
        let active_buff_ids: Vec<i32> = self.active_buffs.iter().map(|x| x.buff_id).collect();

        for active_buff_id in active_buff_ids {
            self.remove_active_buff_internal(active_buff_id);
        }

        PlayerState::collect_stats_with_uncommited_buffs(ctx, &self);
    }

    // Returns the number of seconds left for the buff, or -1 if it's persistent.
    pub fn buff_remaining_time(&self, buff_id: i32, now: Timestamp) -> i32 {
        if let Some(index) = self.buff_index(buff_id) {
            if index < self.active_buffs.len() {
                let buff_state = self.active_buffs.get(index).unwrap();
                if buff_state.buff_duration < 0 {
                    return -1;
                }
                let start_time_stamp = buff_state.buff_start_timestamp.value;
                let buff_end_time = start_time_stamp + buff_state.buff_duration;
                let now = unix(now) as i32;
                if now < buff_end_time {
                    return buff_end_time - now;
                }
                // (else) expired
            }
        }
        0
    }

    pub fn active_buff_id(&self, buff_id: i32) -> Option<&ActiveBuff> {
        self.active_buffs.iter().find(|b| b.buff_id == buff_id)
    }

    pub fn active_buff_of_type(&self, ctx: &ReducerContext, buff_type: i32) -> Option<&ActiveBuff> {
        // Each `BuffType` will have only a small number of `BuffDesc`s.
        // We expect between 1 and 5.
        // Players will usually have relatively few active buffs,
        // but that's not statically bounded.
        // As such, slurping all of the `BuffDesc`s for this type into WASM before the loop
        // results in predictably few bytes crossing the WASM boundary.
        let buff_ids_for_type = ctx
            .db
            .buff_desc()
            .buff_type_id()
            .filter(&buff_type)
            .map(|desc| desc.id)
            .collect::<Vec<_>>();

        for buff in &self.active_buffs {
            if buff_ids_for_type.contains(&buff.buff_id) && self.has_active_buff(buff.buff_id, ctx.timestamp) {
                return Some(buff);
            }
        }
        None
    }

    pub fn active_buff_of_category(&self, ctx: &ReducerContext, buff_category: BuffCategory) -> Option<&ActiveBuff> {
        // Each `BuffType` will have only a small number of `BuffDesc`s.
        // We expect between 1 and 5.
        // Players will usually have relatively few active buffs,
        // but that's not statically bounded.
        // As such, slurping all of the `BuffDesc`s for this type into WASM before the loop
        // results in predictably few bytes crossing the WASM boundary.
        let buff_category = buff_category as i32;
        let buff_ids_for_type = ctx
            .db
            .buff_type_desc()
            .category()
            .filter(buff_category)
            .flat_map(|bt| ctx.db.buff_desc().buff_type_id().filter(bt.id))
            .map(|desc| desc.id)
            .collect::<Vec<_>>();

        for buff in &self.active_buffs {
            if buff_ids_for_type.contains(&buff.buff_id) && self.has_active_buff(buff.buff_id, ctx.timestamp) {
                return Some(buff);
            }
        }
        None
    }

    pub fn has_active_buff(&self, buff_id: i32, now: Timestamp) -> bool {
        self.buff_remaining_time(buff_id, now) != 0
    }

    pub fn pause_all_buffs(&mut self, ctx: &ReducerContext) {
        for active_buff in &mut self.active_buffs {
            if let Some(buff_description) = ctx.db.buff_desc().id().find(&active_buff.buff_id) {
                if buff_description.online_timestamp {
                    let mut timestamp = active_buff.buff_start_timestamp.clone();
                    timestamp.pause(ctx.timestamp);
                    active_buff.buff_start_timestamp = timestamp;
                }
            }
        }
    }

    pub fn restart_all_buffs(&mut self, ctx: &ReducerContext) {
        let mut resumed_timed_buffs = Vec::new();

        for active_buff in &mut self.active_buffs {
            if let Some(buff_description) = ctx.db.buff_desc().id().find(&active_buff.buff_id) {
                if buff_description.online_timestamp {
                    let mut timestamp = active_buff.buff_start_timestamp.clone();
                    timestamp.restart(ctx.timestamp);
                    active_buff.buff_start_timestamp = timestamp;

                    if active_buff.buff_duration >= 0 {
                        resumed_timed_buffs.push(active_buff.buff_id);
                    }
                }
            }
        }

        if ctx.db.player_state().entity_id().find(self.entity_id).is_some() {
            for buff_id in resumed_timed_buffs {
                let remaining_duration = self.buff_remaining_time(buff_id, ctx.timestamp);
                if remaining_duration > 0 {
                    ctx.db
                        .collect_stats_timer()
                        .try_insert(CollectStatsTimer {
                            scheduled_id: 0,
                            scheduled_at: now_plus_secs_f32(remaining_duration as f32 + 0.5, ctx.timestamp),
                            entity_id: self.entity_id,
                        })
                        .ok()
                        .unwrap();
                }
            }
        }
    }

    pub fn has_innerlight_buff(&self, ctx: &ReducerContext) -> bool {
        self.has_active_buff(
            BuffDesc::find_by_buff_category_single(ctx, BuffCategory::InnerLight).unwrap().id,
            ctx.timestamp,
        )
    }

    pub fn set_innerlight_buff(&mut self, ctx: &ReducerContext, duration: i32) -> bool {
        let buff_id = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::InnerLight).unwrap().id;
        if self.buff_remaining_time(buff_id, ctx.timestamp) < duration {
            self.add_active_buff_with_data(ctx, buff_id, Some(duration), None);
            return true;
        }
        false
    }

    pub fn collect_buff_stats(&self, ctx: &ReducerContext, bonuses: &mut HashMap<CharacterStatType, (f32, f32)>) {
        for buff in &self.active_buffs {
            if self.has_active_buff(buff.buff_id, ctx.timestamp) {
                if let Some(buff_description) = ctx.db.buff_desc().id().find(&buff.buff_id) {
                    for (i, stat) in buff_description.stats.iter().enumerate() {
                        // Older persisted buffs can have fewer values than the current buff stat list, fall back to static-data defaults instead of panicking on out-of-bounds access.
                        let value = buff.values.get(i).copied().unwrap_or(stat.value);
                        let entry = bonuses.entry(stat.id).or_insert((0.0, 0.0));
                        if stat.is_pct {
                            *entry = (entry.0, entry.1 + value);
                        } else {
                            *entry = (entry.0 + value, entry.1);
                        }
                    }
                }
            }
        }
    }

    pub fn get_enemy_stat(
        collected_stats: &HashMap<CharacterStatType, (f32, f32)>,
        character_stat_type: CharacterStatType,
        base_value: i32,
    ) -> f32 {
        let mut value = base_value as f32;
        if let Some(arm) = collected_stats.get(&character_stat_type) {
            value += arm.0;
            value *= 1.0 + arm.1;
        }
        value
    }

    pub fn get_enemy_health_regen(
        ctx: &ReducerContext,
        enemy_entity_id: u64,
        regen_buffs: &Vec<i32>,
        regen_buffs_values: &Vec<f32>,
    ) -> f32 {
        let mut health_regen = 0.0;
        if let Some(active_buff_state) = ctx.db.active_buff_state().entity_id().find(enemy_entity_id) {
            for buff_index in active_buff_state.active_buffs.into_iter().filter_map(|b| {
                if b.expired(ctx.timestamp) {
                    None
                } else {
                    regen_buffs.iter().position(|id| b.buff_id == *id)
                }
            }) {
                health_regen += regen_buffs_values[buff_index];
            }
        }
        health_regen
    }

    pub fn collect_enemy_stats(ctx: &ReducerContext, enemy_entity_id: u64) -> HashMap<CharacterStatType, (f32, f32)> {
        let enemy_buffs = ctx.db.active_buff_state().entity_id().find(enemy_entity_id).unwrap();
        let mut collected_stats = HashMap::new();
        enemy_buffs.collect_buff_stats(ctx, &mut collected_stats);
        collected_stats
    }
}
