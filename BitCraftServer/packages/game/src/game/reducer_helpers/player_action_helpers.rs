use std::time::Duration;

use crate::game::coordinates::*;
use crate::game::entities::buff;
//use crate::game::entities::experience_stack::ExperienceStack;
use crate::game::handlers::server::player_clear_action_state::player_clear_action_state;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::*;
use crate::messages::game_util::ActiveBuff;
use crate::messages::static_data::BuffCategory;
use crate::{cargo_desc, player_action_desc, unwrap_or_err, BuffDesc};
use spacetimedb::{log, ReducerContext};

pub fn validate_action_elevation(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    player_coord: FloatHexTile,
    target_coords: SmallHexTile,
    ignore_submerged: bool,
    max_elevation: i16,
    error_verb: &str,
) -> Result<(), String> {
    let player_elevation = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &LargeHexTile::from(player_coord)),
        "Couldn't get player chunk"
    )
    .elevation;
    let target_cell = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &LargeHexTile::from(target_coords)),
        "Couldn't get target chunk"
    );
    if ignore_submerged && target_cell.is_submerged() {
        // note: We might want to use the surface elevation instead so we can't fish from an elevation delta of 50
        return Ok(());
    }
    let target_elevation = target_cell.elevation;
    validate_action_elevation_known(player_elevation, target_elevation, max_elevation, error_verb)
}

pub fn validate_action_elevation_known(
    player_elevation: i16,
    target_elevation: i16,
    max_elevation: i16,
    error_verb: &str,
) -> Result<(), String> {
    if (player_elevation - target_elevation).abs() > max_elevation {
        return Err(format!("Can't {{0}} over a cliff.|~{error_verb}"));
    }
    Ok(())
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
pub fn start_action(
    ctx: &ReducerContext,
    actor_id: u64,
    action_type: PlayerActionType,
    target: Option<u64>,
    recipe_id: Option<i32>,
    duration: Duration,
    result: Result<(), String>,
    timestamp: u64,
) -> Result<(), String> {
    // Upperbody actions can be disallowed depending on what the base layer is doing.
    let layer = action_type.get_layer(ctx);
    if layer == PlayerActionLayer::UpperBody {
        let player_action_base_state = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &actor_id, &PlayerActionLayer::Base),
            "Invalid player id"
        );
        let base_action_type = player_action_base_state.action_type;
        if base_action_type != PlayerActionType::None && player_action_base_state.last_action_result == PlayerActionResult::Success {
            let base_action_id = base_action_type as i32;
            let action_desc = unwrap_or_err!(
                ctx.db.player_action_desc().action_type_id().find(action_type as i32 ),
                "Cannot find allowable action matrix"
            );
            if !action_desc.allowed_concurrent_action_ids.contains(&base_action_id) {
                let error_msg = format!("Cannot {{0}} during {{1}}|~{:?}|~{:?}", action_type, base_action_type);
                return fail_action(actor_id, layer, error_msg);
            }
        }
    } else {
        // If this is a base layer action, but we're doing an incompatible upper layer action at the same time, cancel the upper layer action.
        let mut player_action_upper_state = unwrap_or_err!(
            PlayerActionState::get_state(ctx, &actor_id, &PlayerActionLayer::UpperBody),
            "Invalid player id"
        );
        let upper_action_type = player_action_upper_state.action_type;
        if upper_action_type != PlayerActionType::None {
            let upper_action_id = upper_action_type as i32;
            let upper_action_desc = unwrap_or_err!(
                ctx.db.player_action_desc().action_type_id().find(upper_action_id),
                "Cannot find allowable action matrix"
            );
            if !upper_action_desc.allowed_concurrent_action_ids.contains(&(action_type as i32)) {
                player_action_upper_state.last_action_result = PlayerActionResult::Cancel;
                player_action_upper_state.client_cancel = false;
                player_action_upper_state.action_type = PlayerActionType::None;
                ctx.db.player_action_state().auto_id().update(player_action_upper_state);
            }
        }
    }

    if let Err(msg) = result {
        return fail_action(actor_id, layer, msg);
    } else {
        PlayerActionState::success(
            ctx,
            actor_id,
            action_type,
            layer,
            duration.as_millis() as u64,
            target,
            recipe_id,
            timestamp,
        );
    }
    result
}

pub fn fail_action(actor_id: u64, layer: PlayerActionLayer, error: String) -> Result<(), String> {
    spacetimedb::volatile_nonatomic_schedule_immediate!(player_clear_action_state(
        actor_id,
        PlayerActionType::None,
        layer,
        PlayerActionResult::Fail
    ));
    Err(error)
}

pub fn fail_timing(ctx: &ReducerContext, actor_id: u64, current_action: PlayerActionType, error: String) -> Result<(), String> {
    spacetimedb::volatile_nonatomic_schedule_immediate!(player_clear_action_state(
        actor_id,
        current_action,
        current_action.get_layer(ctx),
        PlayerActionResult::TimingFail
    ));
    Err(error)
}

pub fn schedule_clear_player_action(actor_id: u64, layer: PlayerActionLayer, result: Result<(), String>) -> Result<(), String> {
    let action_result = match result {
        Err(_) => PlayerActionResult::Fail,
        Ok(_) => PlayerActionResult::Success,
    };
    spacetimedb::volatile_nonatomic_schedule_immediate!(player_clear_action_state(actor_id, PlayerActionType::None, layer, action_result));

    result
}

pub fn schedule_clear_player_action_on_err(actor_id: u64, layer: PlayerActionLayer, result: Result<(), String>) -> Result<(), String> {
    if result.is_err() {
        spacetimedb::volatile_nonatomic_schedule_immediate!(player_clear_action_state(
            actor_id,
            PlayerActionType::None,
            layer,
            PlayerActionResult::Fail
        ));
    }

    result
}

pub fn post_reducer_update_cargo(ctx: &ReducerContext, player_entity_id: u64) {
    let carried_cargo_id = InventoryState::get_player_cargo_id(ctx, player_entity_id);
    let carries_cargo = carried_cargo_id != 0;
    let has_cargo_debuff = ActiveBuff::has_cargo_debuff(ctx, player_entity_id);
    let cargo_debuff_id = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::CarryCargo).unwrap().id;
    if !carries_cargo && has_cargo_debuff {
        if buff::deactivate(ctx, player_entity_id, cargo_debuff_id).is_err() {
            log::error!("Unable to deactivate cargo buff for entity {}", player_entity_id);
        }
    } else if carries_cargo {
        let movement_modifier = ctx.db.cargo_desc().id().find(carried_cargo_id).unwrap().movement_modifier;
        if has_cargo_debuff {
            // Make sure the cargo debuff strength is the same as before
            let stats = ctx.db.active_buff_state().entity_id().find(player_entity_id).unwrap();
            let value = stats.active_buff_id(cargo_debuff_id).unwrap().values[0];
            if value != movement_modifier {
                let values = Some(vec![movement_modifier]);
                // new buff of similar duration will override the previous one
                if buff::activate(ctx, player_entity_id, cargo_debuff_id, None, values).is_err() {
                    log::error!("Unable to activate cargo buff for entity {}", player_entity_id);
                }
            }
        } else {
            let values = Some(vec![movement_modifier]);
            if buff::activate(ctx, player_entity_id, cargo_debuff_id, None, values).is_err() {
                log::error!("Unable to activate cargo buff for entity {}", player_entity_id);
            }
            //buff::deactivate_sprint(ctx, player_entity_id);
        }
    }
}
