use spacetimedb::{log, ReducerContext, Table};

use crate::{
    messages::{
        components::{CharacterStatsState, SatiationState},
        static_data::*,
    },
    satiation_state, starving_player_state, StarvingPlayerState,
};

use super::buff;

impl SatiationState {
    pub fn increase_player_satiation(ctx: &ReducerContext, player_entity_id: u64, value: f32) -> bool {
        if value < 0f32 {
            panic!("Cannot increase satiation by a negative value.");
        }

        if value < 0.0001f32 {
            return false;
        }

        if let Some(mut statiation_state) = ctx.db.satiation_state().entity_id().find(player_entity_id) {
            let new_value = (statiation_state.satiation + value).min(Self::get_player_max_satiation(ctx, player_entity_id));

            if statiation_state.satiation == new_value {
                return false;
            }

            if statiation_state.satiation == 0f32 {
                Self::deactivate_starving_debuff(ctx, player_entity_id);
            }

            statiation_state.satiation = new_value;
            ctx.db.satiation_state().entity_id().update(statiation_state);
            return true;
        }
        false
    }

    pub fn decrease_player_satiation(ctx: &ReducerContext, player_entity_id: u64, value: f32) -> bool {
        if value < 0f32 {
            panic!("Cannot decrease satiation by a negative value.");
        }

        if value < 0.0001f32 {
            return false;
        }

        if let Some(mut statiation_state) = ctx.db.satiation_state().entity_id().find(player_entity_id) {
            let new_value = (statiation_state.satiation - value).max(0f32);

            if statiation_state.satiation == new_value {
                return false;
            }

            if new_value == 0f32 {
                Self::activate_starving_debuff(ctx, player_entity_id);
            }

            statiation_state.satiation = new_value;
            ctx.db.satiation_state().entity_id().update(statiation_state);
            return true;
        }
        false
    }

    pub fn add_player_satiation(ctx: &ReducerContext, player_entity_id: u64, value: f32) -> bool {
        if value < 0f32 {
            return Self::decrease_player_satiation(ctx, player_entity_id, value.abs());
        }

        Self::increase_player_satiation(ctx, player_entity_id, value)
    }

    pub fn get_player_max_satiation(ctx: &ReducerContext, player_entity_id: u64) -> f32 {
        CharacterStatsState::get_entity_stat(ctx, player_entity_id, CharacterStatType::MaxSatiation)
    }

    fn activate_starving_debuff(ctx: &ReducerContext, player_entity_id: u64) {
        if let Some(starving_debuff) = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::Starving) {
            let _ = buff::activate(ctx, player_entity_id, starving_debuff.id, None, None);

            let starving_player_state = StarvingPlayerState {
                entity_id: player_entity_id,
            };

            if ctx.db.starving_player_state().try_insert(starving_player_state).is_err() {
                log::error!("Failed to insert StarvingPlayerState");
            }
        }
    }

    fn deactivate_starving_debuff(ctx: &ReducerContext, player_entity_id: u64) {
        if let Some(starving_debuff) = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::Starving) {
            let _ = buff::deactivate(ctx, player_entity_id, starving_debuff.id);

            if !ctx.db.starving_player_state().entity_id().delete(player_entity_id) {
                log::error!("Failed to delete StarvingPlayerState");
            }
        }
    }
}
