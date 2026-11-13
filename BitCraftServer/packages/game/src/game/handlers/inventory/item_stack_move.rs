use crate::building_desc;
use crate::cargo_desc;
use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::entities::inventory_type::InventoryType;
use crate::game::game_state;
use crate::game::reducer_helpers::loot_chest_helpers;
use crate::game::reducer_helpers::player_action_helpers;
use crate::messages::action_request::PlayerItemStackMoveRequest;
use crate::messages::components::*;
use crate::messages::game_util::*;
use crate::unwrap_or_err;
use crate::BuildingCategory;
use crate::BuildingFunction;
use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use super::inventory_helper;

#[spacetimedb::reducer]
#[feature_gate]
pub fn item_stack_move(ctx: &ReducerContext, request: PlayerItemStackMoveRequest) -> Result<(), String> {
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

    if target_inventory.owner_entity_id > 0 {
        if let Some(building_state) = ctx.db.building_state().entity_id().find(target_inventory.owner_entity_id) {
            let desc = ctx.db.building_desc().id().find(building_state.building_description_id).unwrap();
            if !desc.has_category(ctx, BuildingCategory::Storage)
                && !desc.has_category(ctx, BuildingCategory::TradingPost)
                && !desc.has_category(ctx, BuildingCategory::Barter)
                && !desc.has_category(ctx, BuildingCategory::RentTerminal)
                && !desc.has_category(ctx, BuildingCategory::Bank)
                && !desc.has_category(ctx, BuildingCategory::RegionalBank)
            {
                return Err("Cannot store items in this building".into());
            }
        }
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

    let pocket_index = request.from_pocket.pocket_index;

    let item_stack = unwrap_or_err!(
        source_inventory.get_pocket_contents(pocket_index as usize),
        "No items in this pocket: entity_id: {{0}} pocket_index: {{1}}|~{}|~{}",
        source_inventory.entity_id,
        pocket_index
    );

    let quantity = if request.quantity > 0 {
        request.quantity
    } else {
        item_stack.quantity
    };

    let mut deposit_stack = item_stack.clone_with_quantity(quantity);

    let mut forced_target_index = None;
    let mut id_per_slot = Vec::new();

    if item_stack.item_type == ItemType::Cargo {
        inventory_helper::validate_cargo_target(ctx, &target_inventory, &deposit_stack)?;

        if let Some(building) = ctx.db.building_state().entity_id().find(target_inventory.owner_entity_id) {
            let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
            if let Some(function) = BuildingFunction::from_inventory_index(&building_desc, target_inventory.inventory_index) {
                // Typed storage, each slot can only receive a matching cargo id
                // let's see if any of those typed slots matches the type of cargo we're dropping
                id_per_slot = function.allowed_item_id_per_slot.clone();

                for (i, required_id) in id_per_slot.iter().enumerate() {
                    if *required_id == item_stack.item_id {
                        forced_target_index = Some(i);
                    }
                }
                if forced_target_index.is_none() && !function.allowed_item_id_per_slot.is_empty() {
                    let cargo_name = ctx.db.cargo_desc().id().find(item_stack.item_id).unwrap().name;
                    let err_msg = format!("You cannot put {{0}} in this storage.|~{}", cargo_name);
                    return Err(err_msg);
                }
            }
        }
    }

    let mut discovery = Discovery::new(actor_id);
    discovery.acquire_item_stack(ctx, &deposit_stack);

    if let Some(forced_target_index) = forced_target_index {
        if !target_inventory.add_at(ctx, forced_target_index, deposit_stack) {
            return Err("Not enough room in storage".into());
        }
        // add_at will still try to overflow to the next slot. Let's confirm that all the items are still valid, in case there are
        // consecutive duplicate ids (in which case the overflow could be legal).
        for (i, required_id) in id_per_slot.iter().enumerate() {
            if let Some(content) = target_inventory.get_at(i) {
                if content.item_id != *required_id {
                    return Err("not enough room in this slot to add more".into());
                }
            }
        }

        deposit_stack.quantity -= quantity;
    } else {
        InventoryState::add_partial_to_inventory_and_discover(
            ctx,
            target_inventory.owner_entity_id,
            &mut discovery,
            &mut target_inventory,
            &mut deposit_stack,
            false,
        );
    }

    if deposit_stack.quantity == quantity {
        if target_inventory.owner_entity_id == actor_id {
            return Err("Not enough space in your inventory!".into());
        }
        return Err("Not enough room in storage!".into());
    }

    discovery.commit(ctx);

    let quantity_left = quantity - deposit_stack.quantity;
    if quantity_left < 0 {
        return Err("That quantity is no longer available.".into());
    }

    let mut stack = deposit_stack;
    stack.quantity = quantity - deposit_stack.quantity;
    ActionLogState::log_storage(
        ctx,
        &source_inventory,
        &target_inventory,
        actor_id,
        source_inventory_type,
        target_inventory_type,
        &stack,
    );

    if source_inventory.remove_quantity_at(pocket_index as usize, quantity_left).is_none() {
        return Err("The pocket no longer has enough quantity.".into());
    }

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

    // TODO: Allow deposit from or into toolbelt
    ctx.db.inventory_state().entity_id().update(target_inventory);

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
