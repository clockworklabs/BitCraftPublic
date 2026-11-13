use bitcraft_macro::feature_gate;
use crate::game::discovery::Discovery;
use crate::game::game_state;
use crate::game::reducer_helpers::player_action_helpers::post_reducer_update_cargo;
use crate::messages::action_request::PlayerDroppedInventoryPickUpRequest;
use crate::messages::components::*;
use crate::unwrap_or_err;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_pick_up(ctx: &ReducerContext, request: PlayerDroppedInventoryPickUpRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let dropped_inventory = unwrap_or_err!(
        ctx.db
            .dropped_inventory_state()
            .entity_id()
            .find(request.dropped_inventory_entity_id),
        "Items not found"
    );

    let dropped_inventory_coordinates = dropped_inventory.validate_interact_and_get_inventory_coordinates(ctx, actor_id)?;

    let inventory = dropped_inventory.inventory(ctx);
    let mut item_stack = unwrap_or_err!(inventory.get_pocket_contents(request.pocket_index as usize), "Items state changed");
    if item_stack.item_id != request.item_id {
        return Err("State of items changed".into());
    }

    let mut discovery = Discovery::new(actor_id);
    let previous_quantity = item_stack.quantity;

    if request.to_deployable {
        let item_stack_vec = vec![item_stack];
        let removed_quantity = item_stack.quantity;
        // We need to update the inventories BEFORE doing the all-size-fit-one inventory call below, in case we fail to acquire a single-cargo item pile
        // into nearby buildings or deployables - in which case it would be dropped back into the pile before being deleted.
        update_inventories(
            ctx,
            actor_id,
            inventory,
            dropped_inventory,
            request.pocket_index as usize,
            removed_quantity,
        );

        InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            actor_id,
            &item_stack_vec,
            |x| dropped_inventory_coordinates.distance_to(x),
            true,
            || vec![dropped_inventory_coordinates],
            true,
        )?;
    } else {
        if !InventoryState::add_partial_and_discover(ctx, actor_id, &mut discovery, &mut item_stack) {
            return Err("Failed to pickup items, unable to update player inventory!".into());
        }
        let removed_quantity = previous_quantity - item_stack.quantity;
        if removed_quantity == 0 {
            return Err("No room in inventory to pick-up an item".into());
        }
        update_inventories(
            ctx,
            actor_id,
            inventory,
            dropped_inventory,
            request.pocket_index as usize,
            removed_quantity,
        );
    }
    discovery.commit(ctx);

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

fn update_inventories(
    ctx: &ReducerContext,
    actor_id: u64,
    mut inventory: InventoryState,
    dropped_inventory: DroppedInventoryState,
    pocket_index: usize,
    removed_quantity: i32,
) {
    inventory.remove_quantity_at(pocket_index, removed_quantity);
    dropped_inventory.on_inventory_updated(ctx, actor_id, inventory);
}
