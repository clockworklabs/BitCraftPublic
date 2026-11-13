use bitcraft_macro::feature_gate;
use crate::game::game_state;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::PlayerItemStackSplitRequest;
use crate::messages::components::*;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

use super::inventory_helper;

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_stack_split(ctx: &ReducerContext, request: PlayerItemStackSplitRequest) -> Result<(), String> {
    if request.new_stack_count <= 0 {
        return Err("Cannot split into an empty stack".into());
    }

    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut source_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(request.from_pocket.inventory_entity_id),
        "Invalid source inventory"
    );

    let player_location = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Player has no location").coordinates();

    let source_inventory_type = inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        source_inventory.owner_entity_id,
        source_inventory.player_owner_entity_id,
    )?;

    inventory_helper::validate_split(&source_inventory_type)?;

    let from_pocket_index = request.from_pocket.pocket_index;

    let item_stack = unwrap_or_err!(
        source_inventory.get_pocket_contents(from_pocket_index as usize),
        "No items in this pocket"
    );

    let quantity = item_stack.quantity;

    let quantity_left = quantity - request.new_stack_count;
    if quantity_left < 0 {
        return Err("That quantity is no longer available".into());
    }

    let new_pocket_index = unwrap_or_err!(source_inventory.next_free_pocket(item_stack.item_type), "No empty pockets");

    let new_stack = item_stack.clone_with_quantity(request.new_stack_count);

    // To verify what users saw on the UI and intended to split is the same as what the serve sees and will move
    if new_stack.item_id != request.item_id {
        return Err("Item to split mismatch. Please try again.".into());
    }

    source_inventory.remove_quantity_at(from_pocket_index as usize, request.new_stack_count);
    source_inventory.set_at(new_pocket_index, Some(new_stack));
    // player_inventory.pockets[new_pocket_index].set(item_id, item_type, inserted_count, durability);

    ctx.db.inventory_state().entity_id().update(source_inventory);

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
