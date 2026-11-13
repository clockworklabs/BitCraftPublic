use super::inventory_helper;
use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::entities::inventory_type::InventoryType;
use crate::game::game_state;
use crate::game::reducer_helpers::loot_chest_helpers;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::PlayerItemStackMoveAllRequest;
use crate::messages::components::*;
use crate::unwrap_or_err;
use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_stack_move_all(ctx: &ReducerContext, request: PlayerItemStackMoveAllRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut source_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(request.from_pocket.inventory_entity_id),
        "Invalid source inventory"
    );

    let mut target_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(request.to_pocket.inventory_entity_id),
        "Invalid target inventory"
    );

    // This reducer is used for transferring an item stack from one inventory to another - UI on the client prevents same-inventory transfer.
    // pocket_swap_contents is generally used for same (or different) inventory transfers, possibly swapping an empty pocket into the original one.
    if source_inventory.entity_id == target_inventory.entity_id {
        return Err("You cannot use this reducer on the same inventory instance".into());
    }

    let player_location = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Player has no location").coordinates();

    let source_inventory_type = inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        source_inventory.owner_entity_id,
        source_inventory.player_owner_entity_id,
    )?;

    let target_inventory_type = inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        target_inventory.owner_entity_id,
        target_inventory.player_owner_entity_id,
    )?;

    inventory_helper::validate_move(&target_inventory_type)?;

    let source_item = unwrap_or_err!(
        source_inventory.get_pocket_contents(request.from_pocket.pocket_index as usize),
        "No items in this pocket: entity_id: {{0}} pocket_index: {{1}}|~{}|~{}",
        source_inventory.entity_id,
        request.from_pocket.pocket_index
    );

    let wallet_pocket = target_inventory.wallet_pocket_target(ctx, &source_item);
    // target inventory is overridden by wallet if we're moving a hexcoin item stack into a player inventory from a non-wallet source
    if wallet_pocket.is_some()
        && !(source_inventory.owner_entity_id == target_inventory.owner_entity_id && source_inventory.inventory_index != 2)
    {
        target_inventory = InventoryState::get_player_wallet(ctx, target_inventory.owner_entity_id).unwrap();

        // Because the target inventory has changed, we need to check this again
        if source_inventory.entity_id == target_inventory.entity_id {
            return Err("You cannot use this reducer on the same inventory instance".into());
        }
    }
    inventory_helper::validate_cargo_target(ctx, &target_inventory, &source_item)?;

    if !target_inventory.fits(ctx, source_item.clone_with_quantity(1)) {
        return Err("~Target inventory is full".into());
    }

    let mut added_quantities: Vec<(usize, i32)> = Vec::new();
    let mut discovery = Discovery::new(actor_id);

    let target_inventory_owner_entity_id = target_inventory.owner_entity_id;

    for (index, pocket) in source_inventory.pockets.iter().enumerate() {
        let mut contents = match pocket.contents {
            Some(value) => value,
            None => continue,
        };

        if contents.item_id != source_item.item_id {
            continue;
        }

        let original_quantity = contents.quantity;

        if let Some(wallet_pocket_index) = wallet_pocket {
            target_inventory.add_at(ctx, wallet_pocket_index, contents);
            contents.quantity = 0;
        } else {
            InventoryState::add_partial_to_inventory_and_discover(
                ctx,
                target_inventory.owner_entity_id,
                &mut discovery,
                &mut target_inventory,
                &mut contents,
                target_inventory_owner_entity_id == actor_id,
            );
        }

        // Exit early if nothing was added
        if contents.quantity == original_quantity {
            break;
        }

        added_quantities.push((index, original_quantity - contents.quantity));

        let mut stack = contents;
        stack.quantity = original_quantity - contents.quantity;
        ActionLogState::log_storage(
            ctx,
            &source_inventory,
            &target_inventory,
            actor_id,
            source_inventory_type,
            target_inventory_type,
            &stack,
        );
    }

    for (index, quantity) in added_quantities {
        source_inventory.remove_quantity_at(index, quantity);
    }

    discovery.commit(ctx);

    if source_inventory_type == InventoryType::LootChest && source_inventory.is_empty() {
        loot_chest_helpers::on_item_taken_from_loot_chest(ctx, source_inventory.owner_entity_id, true)?;
    }

    if source_inventory_type == InventoryType::Dropped {
        let dropped_inventory = ctx
            .db
            .dropped_inventory_state()
            .entity_id()
            .find(source_inventory.owner_entity_id)
            .unwrap();
        dropped_inventory.on_inventory_updated(ctx, actor_id, source_inventory);
    } else {
        ctx.db.inventory_state().entity_id().update(source_inventory);
    }

    ctx.db.inventory_state().entity_id().update(target_inventory);
    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
