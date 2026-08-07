use spacetimedb::{log, ReducerContext};

use crate::game::reducer_helpers::{move_validation_helpers, player_action_helpers};
use crate::{game::game_state, messages::components::*, unwrap_or_err};

impl PlayerActionState {
    pub fn get_state(ctx: &ReducerContext, entity_id: &u64, layer: &PlayerActionLayer) -> Option<PlayerActionState> {
        return ctx
            .db
            .player_action_state()
            .entity_id()
            .filter(entity_id)
            .find(|x| x.layer == *layer);
    }

    pub fn get_state_for_action_type(ctx: &ReducerContext, entity_id: &u64, action_type: &PlayerActionType) -> Option<PlayerActionState> {
        let layer = action_type.get_layer(ctx);
        PlayerActionState::get_state(ctx, entity_id, &layer)
    }

    pub fn is_player_doing_action(ctx: &ReducerContext, entity_id: &u64, action_type: &PlayerActionType) -> Result<bool, String> {
        let player_action_state = unwrap_or_err!(
            PlayerActionState::get_state_for_action_type(ctx, entity_id, action_type),
            "Player has no action state"
        );
        Ok(player_action_state.action_type == *action_type)
    }

    pub fn get_auto_id(ctx: &ReducerContext, entity_id: &u64, layer: &PlayerActionLayer) -> Option<u64> {
        return PlayerActionState::get_state(ctx, &entity_id, &layer).map(|state| state.auto_id);
    }

    pub fn update_by_entity_id(ctx: &ReducerContext, entity_id: &u64, mut state: PlayerActionState) -> Result<(), String> {
        let id: u64 = unwrap_or_err!(
            PlayerActionState::get_auto_id(ctx, &entity_id, &state.layer),
            "Can't find base layer state, invalid player id."
        );
        state.auto_id = id;
        ctx.db.player_action_state().auto_id().update(state);
        Ok(())
    }

    pub fn clear_by_entity_id(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
        let chunk_index = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap().chunk_index;
        PlayerActionState::update_by_entity_id(
            ctx,
            &entity_id,
            PlayerActionState {
                auto_id: 0,
                entity_id,
                action_type: PlayerActionType::None,
                layer: PlayerActionLayer::Base,
                last_action_result: PlayerActionResult::Fail,
                start_time: game_state::unix_ms(ctx.timestamp),
                duration: 0,
                target: None,
                recipe_id: None,
                client_cancel: false,
                was_consumed: false,
                chunk_index,
                _pad1: 0,
                _pad2: 0,
                _pad3: 0,
            },
        )?;

        PlayerActionState::update_by_entity_id(
            ctx,
            &entity_id,
            PlayerActionState {
                auto_id: 0,
                entity_id,
                action_type: PlayerActionType::None,
                layer: PlayerActionLayer::UpperBody,
                last_action_result: PlayerActionResult::Fail,
                start_time: game_state::unix_ms(ctx.timestamp),
                duration: 0,
                target: None,
                recipe_id: None,
                client_cancel: false,
                was_consumed: false,
                chunk_index,
                _pad1: 0,
                _pad2: 0,
                _pad3: 0,
            },
        )
    }

    pub fn mark_as_consumed(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
        let mut a = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &entity_id, &PlayerActionLayer::Base),
            "Player doesn't have Base PlayerActionState"
        );
        a.was_consumed = true;
        a.start_time = a.start_time.saturating_add(a.duration - a.duration / 10);
        ctx.db.player_action_state().auto_id().update(a);

        let mut a = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &entity_id, &PlayerActionLayer::UpperBody),
            "Player doesn't have UpperBody PlayerActionState"
        );
        a.was_consumed = true;
        a.start_time = a.start_time.saturating_add(a.duration - a.duration / 10);
        ctx.db.player_action_state().auto_id().update(a);

        Ok(())
    }

    pub fn validate_action_timing(
        ctx: &ReducerContext,
        entity_id: u64,
        action_type: PlayerActionType,
        timestamp: u64,
    ) -> Result<(), String> {
        let layer = action_type.get_layer(ctx);
        let player_action = unwrap_or_err!(PlayerActionState::get_state(ctx, &entity_id, &layer), "Player has no ActionState");

        if player_action.last_action_result == PlayerActionResult::Fail || player_action.last_action_result == PlayerActionResult::Cancel {
            return Ok(());
        }

        move_validation_helpers::validate_move_timestamp(player_action.start_time, timestamp, ctx.timestamp)?;

        let elapsed_normalized = (timestamp - player_action.start_time) as f32 / player_action.duration as f32;
        if elapsed_normalized >= 0.95f32 {
            return Ok(());
        }

        if elapsed_normalized >= 0.80f32 {
            return move_validation_helpers::action_validation_strike(ctx, entity_id, action_type);
        }

        player_action_helpers::fail_timing(ctx, entity_id, action_type, format!("Tried to {{0}} too quickly|~{:?}", action_type))
    }

    pub fn validate_timestamp_basic(
        ctx: &ReducerContext,
        entity_id: u64,
        action_type: PlayerActionType,
        timestamp: u64,
    ) -> Result<(), String> {
        let layer = action_type.get_layer(ctx);
        let player_action = unwrap_or_err!(PlayerActionState::get_state(ctx, &entity_id, &layer), "Player has no ActionState");

        return move_validation_helpers::validate_move_timestamp(player_action.start_time, timestamp, ctx.timestamp);
    }

    pub fn validate(ctx: &ReducerContext, actor_id: u64, action_type: PlayerActionType, target: Option<u64>) -> Result<(), String> {
        let player_action = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &actor_id, &action_type.get_layer(ctx)),
            "Invalid player id"
        );

        if player_action.action_type != action_type {
            if player_action.action_type == PlayerActionType::Death {
                // Don't print a message if death interrupted the current action
                return Err(String::new());
            }
            return Err(format!(
                "Invalid action type: received {{0}}, expected {{1}}|~{:?}|~{:?}",
                player_action.action_type, action_type
            )
            .into());
        }

        if player_action.target != target {
            return Err("Invalid action target".into());
        }

        if player_action.was_consumed {
            return Err("Invalid repeat action".into());
        }

        Ok(())
    }

    pub fn success(
        ctx: &ReducerContext,
        entity_id: u64,
        action_type: PlayerActionType,
        layer: PlayerActionLayer,
        duration: u64,
        target: Option<u64>,
        recipe_id: Option<i32>,
        timestamp: u64,
    ) {
        let chunk_index = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap().chunk_index;

        if let Err(e) = PlayerActionState::update_by_entity_id(
            ctx,
            &entity_id,
            PlayerActionState {
                auto_id: 0,
                entity_id: entity_id,
                action_type: action_type,
                layer: layer,
                last_action_result: PlayerActionResult::Success,
                start_time: timestamp,
                duration,
                target,
                recipe_id,
                client_cancel: false,
                was_consumed: false,
                chunk_index,
                _pad1: 0,
                _pad2: 0,
                _pad3: 0,
            },
        ) {
            log::error!("Couldn't call success on PlayerActionState, with error: {}", e);
        }
    }

    pub fn update_chunk_index_on_all_layers(ctx: &ReducerContext, entity_id: u64, chunk_index: u64) {
        // Update chunk index of the other layers as well
        for mut action_state in ctx.db.player_action_state().entity_id().filter(entity_id) {
            if action_state.chunk_index != chunk_index {
                action_state.chunk_index = chunk_index;
                ctx.db.player_action_state().auto_id().update(action_state);
            }
        }
    }
}
