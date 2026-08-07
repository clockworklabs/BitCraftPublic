use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        entities::{building_state::InventoryState, food},
        game_state::{self},
    },
    messages::{action_request::PlayerEatRequest, components::*, static_data::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn eat(ctx: &ReducerContext, request: PlayerEatRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let is_sleeping = PlayerActionState::is_player_doing_action(ctx, &actor_id, &PlayerActionType::Sleep)?;
    if is_sleeping {
        return Err("You cannot eat while you sleep.".into());
    }

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
    let contents = unwrap_or_err!(inventory.get_pocket_contents(request.pocket_index as usize), "Invalid pocket");
    if contents.item_id <= 0 {
        return Err("Nothing to eat".into());
    }

    let food = unwrap_or_err!(ctx.db.food_desc().item_id().find(&contents.item_id), "Item can't be eaten");

    if !food.consumable_while_in_combat && ThreatState::in_combat(ctx, actor_id) {
        return Err("Cannot eat this while in combat".into());
    }

    inventory.remove_quantity_at(request.pocket_index as usize, 1);
    ctx.db.inventory_state().entity_id().update(inventory);

    food::consume(ctx, actor_id, &food, 1)?;

    Ok(())
}
