use bitcraft_macro::feature_gate;
use std::time::Duration;

use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::game::handlers::server::player_clear_action_state;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::{
    game::{discovery::Discovery, entities::building_state::InventoryState, game_state::game_state_filters},
    messages::{action_request::PlayerItemConvertRequest, components::*, game_util::ToolRequirement, static_data::*},
    unwrap_or_err,
};

pub fn event_delay_recipe_id(ctx: &ReducerContext, request: &PlayerItemConvertRequest) -> (Duration, Option<i32>) {
    let conversion_recipe = match ctx
        .db
        .item_conversion_recipe_desc()
        .id()
        .find(&(request.conversion_recipe_id as i32))
    {
        Some(item_conversion_recipe) => item_conversion_recipe,
        None => return (Duration::ZERO, None),
    };

    let time_cost = conversion_recipe.time_cost as f32;
    return (Duration::from_secs_f32(time_cost), Some(conversion_recipe.id));
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn item_convert_start(ctx: &ReducerContext, request: PlayerItemConvertRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let (delay, recipe_id) = event_delay_recipe_id(ctx, &request);
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::ConvertItems,
        None,
        recipe_id,
        delay,
        reduce(ctx, actor_id, &request, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn item_convert(ctx: &ReducerContext, request: PlayerItemConvertRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let layer = PlayerActionType::ConvertItems.get_layer(ctx);
    match reduce(ctx, actor_id, &request, false) {
        Ok(()) => player_clear_action_state::reduce(
            ctx,
            actor_id,
            PlayerActionType::None,
            layer,
            PlayerActionResult::Success,
        ),
        Err(error) => player_action_helpers::schedule_clear_player_action_on_err(actor_id, layer, Err(error)),
    }
}

fn reduce(ctx: &ReducerContext, actor_id: u64, request: &PlayerItemConvertRequest, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::ConvertItems, None)?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::ConvertItems, request.timestamp)?;
    }

    let item_conversion_recipe = unwrap_or_err!(
        ctx.db
            .item_conversion_recipe_desc()
            .id()
            .find(&(request.conversion_recipe_id as i32)),
        "Invalid item conversion recipe!"
    );

    // When the location context of a recipe is not 0 the location context must match the request's/player's location context
    if item_conversion_recipe.location_context != 0 && item_conversion_recipe.location_context != request.location_context as i32 {
        let location_context: ItemConversionLocationContext = match item_conversion_recipe.location_context {
            0 => ItemConversionLocationContext::None,
            1 => ItemConversionLocationContext::Water,
            _default => ItemConversionLocationContext::None,
        };

        return Err(String::from(format!("Cannot perform item conversion, you must be in {{0}}|~{:?}", location_context)).into());
    }

    let player_large_tile = game_state_filters::coordinates_any(ctx, actor_id).parent_large_tile();
    let mut terrain_cache = TerrainChunkCache::empty();

    // If a player isn't in a deployable and is in swim-deep water then they can't do item conversion
    let terrain_source = unwrap_or_err!(terrain_cache.get_terrain_cell(ctx, &player_large_tile), "Invalid location");
    if terrain_source.player_should_swim() && ctx.db.mounting_state().entity_id().find(&actor_id).is_none() {
        return Err("Cannot perform item conversion while swimming".into());
    }

    // Validate Tool Requirement
    if !item_conversion_recipe.allow_use_hands {
        let required_tool = ToolRequirement {
            tool_type: item_conversion_recipe.required_equipment_id,
            level: item_conversion_recipe.required_equipment_tier,
            power: 1,
        };

        let mut _tool: Option<ToolDesc> = match ToolDesc::get_required_tool(ctx, actor_id, &required_tool) {
            Ok(t) => Some(t),
            Err(s) => return Err(s.into()),
        };
    }

    // Check to see if we have all the ingredients for this item conversion
    let mut player_inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    if player_inventory.remove(&item_conversion_recipe.input_items) == false {
        return Err("Failed to convert items: all ingredients were not found in inventory.".into());
    }

    // Apply stamin cost to the player
    let stamina_cost = item_conversion_recipe.stamina_cost as f32;

    let stamina_state = unwrap_or_err!(
        ctx.db.stamina_state().entity_id().find(&actor_id),
        "Player missing stamina component!"
    );
    if stamina_state.stamina < stamina_cost {
        return Err("Not enough stamina.".into());
    }

    // Apply stamina cost for item conversion
    if !dry_run {
        StaminaState::add_player_stamina(ctx, actor_id, -stamina_cost);
        ctx.db.inventory_state().entity_id().update(player_inventory);
    }

    // Try to add the recipe outcome item to the player's inventory
    let mut output_item = unwrap_or_err!(item_conversion_recipe.output_item, "Invalid recipe output item!");

    if !dry_run {
        let mut discovery = Discovery::new(actor_id);
        if !InventoryState::add_partial_and_discover(ctx, actor_id, &mut discovery, &mut output_item) {
            return Err("Failed to convert items: unable to update player state!".into());
        }
        if output_item.quantity > 0 {
            return Err("Not enough room in inventory to convert".into());
        }
        discovery.commit(ctx);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
