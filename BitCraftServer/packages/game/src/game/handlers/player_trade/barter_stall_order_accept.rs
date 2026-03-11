use std::i32;

use spacetimedb::ReducerContext;

use crate::messages::static_data::AchievementDesc;
use crate::{achievement_desc, building_desc, deployable_desc, skill_desc, traveler_trade_order_desc, SmallHexTile};
use crate::{
    game::{
        entities::building_state::{BuildingState, InventoryState},
        game_state,
        reducer_helpers::player_action_helpers,
    },
    messages::{
        action_request::PlayerBarterStallOrderAccept,
        components::*,
        game_util::{ItemStack, ItemType},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn barter_stall_order_accept(ctx: &ReducerContext, request: PlayerBarterStallOrderAccept) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.shop_entity_id, request.trade_order_entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, shop_entity_id: u64, trade_order_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    if ThreatState::in_combat(ctx, entity_id) {
        return Err("Cannot execute a trade order while in combat".into());
    }

    let coordinates = ctx.db.mobile_entity_state().entity_id().find(&entity_id).unwrap().coordinates();
    let building = ctx.db.building_state().entity_id().find(&shop_entity_id);
    let mut deployable_location = None;

    let mut item_index = None;
    let mut cargo_index = None;
    let mut claim = None;

    if let Some(building) = &building {
        if building.distance_to(ctx, &coordinates) > 5 {
            return Err("Too far".into());
        }
        claim = ctx.db.claim_state().entity_id().find(&building.claim_entity_id);

        // These indices will be used to find the inventory index.
        // The Item and Cargo Inventories for buildings are created based on the order of the inventory building features.
        // When finding the indicees, ignore non-inventory building features (eg. BarterStall).
        // Note: There can be a building feature that serves as both of the item and cargo inventories (eg. Bank).
        let description = ctx.db.building_desc().id().find(&building.building_description_id).unwrap();
        item_index = description
            .functions
            .iter()
            .filter(|f| f.storage_slots > 0 || f.cargo_slots > 0)
            .position(|f| f.storage_slots > 0);
        cargo_index = description
            .functions
            .iter()
            .filter(|f| f.storage_slots > 0 || f.cargo_slots > 0)
            .position(|f| f.cargo_slots > 0);
    } else if let Some(deployable) = ctx.db.deployable_state().entity_id().find(&shop_entity_id) {
        if deployable.hidden {
            return Err("Cannot interact with a hidden deployable.".into());
        }

        let location = ctx
            .db
            .mobile_entity_state()
            .entity_id()
            .find(&shop_entity_id)
            .unwrap()
            .coordinates();
        if location.distance_to(coordinates) > 5 {
            return Err("Too far".into());
        }

        deployable_location = Some(location);

        let description = ctx.db.deployable_desc().id().find(&deployable.deployable_description_id).unwrap();
        if description.storage > 0 {
            item_index = Some(0);
        }
        if description.stockpile > 0 {
            cargo_index = Some(0);
        }
    }

    let mut trade_order = unwrap_or_err!(
        ctx.db.trade_order_state().entity_id().find(&trade_order_entity_id),
        "Trade order could not be found!"
    );

    if trade_order.shop_entity_id != shop_entity_id {
        return Err("This trade order can't be accepted here".into());
    }

    // Is this out of stock?
    if trade_order.remaining_stock <= 0 {
        return Err("No more stock available for this trade order!".into());
    }

    let mut offered_item_stacks = Vec::new();
    let mut required_item_stacks = Vec::new();

    //validate traveler trade order requirements
    if trade_order.traveler_trade_order_id != None {
        if let Some(traveler_trade_order) = ctx
            .db
            .traveler_trade_order_desc()
            .id()
            .find(&trade_order.traveler_trade_order_id.unwrap())
        {
            //validate achievement requirements
            for achievement_id in traveler_trade_order.achievement_requirements {
                if !AchievementDesc::meets_requirements_for_achievement(ctx, entity_id, achievement_id) {
                    let achievement = unwrap_or_err!(
                        ctx.db.achievement_desc().id().find(&achievement_id),
                        "Required achievement does not exist"
                    );
                    return Err(format!("You must have completed achievement {{0}} to do this trade|~{}", achievement.name).into());
                }
            }

            //validate skill level requirements
            for level_requirements in traveler_trade_order.level_requirements {
                if !PlayerState::meets_level_requirement(ctx, entity_id, &level_requirements) {
                    let skill = unwrap_or_err!(
                        ctx.db.skill_desc().id().find(&level_requirements.skill_id),
                        "Required skill does not exist."
                    );
                    return Err(format!(
                        "You must have level {{0}} or higher in {{1}} to do this trade|~{}|~{}",
                        level_requirements.level, skill.name
                    )
                    .into());
                }
            }

            // validate knowledges
            if traveler_trade_order.required_knowledges.len() > 0 || traveler_trade_order.blocking_knowledges.len() > 0 {
                let secondary_knowledge = ctx.db.knowledge_secondary_state().entity_id().find(entity_id).unwrap();

                if traveler_trade_order.required_knowledges.len() > 0 {
                    let mut possess_some_knowledges = false;
                    for knowledge_id in &traveler_trade_order.required_knowledges {
                        possess_some_knowledges |= secondary_knowledge
                            .entries
                            .iter()
                            .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired);
                    }
                    if !possess_some_knowledges {
                        return Err("You don't have the necessary knowledge to accept this trade order".into());
                    }
                }

                if traveler_trade_order.blocking_knowledges.len() > 0 {
                    let mut possess_all_knowledges = true;
                    for knowledge_id in &traveler_trade_order.blocking_knowledges {
                        possess_all_knowledges &= secondary_knowledge
                            .entries
                            .iter()
                            .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired);
                    }
                    if possess_all_knowledges {
                        return Err("You already know everything this trade order has to offer".into());
                    }
                }
            }

            // trade order offered items are under the form of item_stack, but they might not be matching (e.g. you can have a stack of 50 axes with durabilities)
            // therefore we need to convert that format into real item stacks with a capacity and durability
            for offered_item in &traveler_trade_order.offer_items {
                let mut item_stack = ItemStack::new(ctx, offered_item.item_id, offered_item.item_type, 1);
                if item_stack.durability.is_some() {
                    for _ in 0..offered_item.quantity {
                        offered_item_stacks.push(item_stack.clone());
                    }
                } else {
                    item_stack.quantity = offered_item.quantity;
                    offered_item_stacks.push(item_stack);
                }
            }

            for required_item in &traveler_trade_order.required_items {
                let mut item_stack = ItemStack::new(ctx, required_item.item_id, required_item.item_type, 1);
                if item_stack.durability.is_some() {
                    for _ in 0..required_item.quantity {
                        required_item_stacks.push(item_stack.clone());
                    }
                } else {
                    item_stack.quantity = required_item.quantity;
                    required_item_stacks.push(item_stack);
                }
            }
        }
    } else {
        // trade order offered items are under the form of item_stack, but they might not be matching (e.g. you can have a stack of 50 axes with durabilities)
        // therefore we need to convert that format into real item stacks with a capacity and durability
        for offered_item in &trade_order.offer_items {
            let mut item_stack = ItemStack::new(ctx, offered_item.item_id, offered_item.item_type, 1);
            if item_stack.durability.is_some() {
                for _ in 0..offered_item.quantity {
                    offered_item_stacks.push(item_stack.clone());
                }
            } else {
                item_stack.quantity = offered_item.quantity;
                offered_item_stacks.push(item_stack);
            }
        }
        required_item_stacks = trade_order.required_items.clone();
    }

    // Prevent accepting trade orders offering only known auto-collect items
    let cancel_trade = !offered_item_stacks.is_empty()
        && offered_item_stacks.iter_mut().all(|item_stack| {
            let (auto_collectable, already_collected) = item_stack.can_auto_collect(ctx, entity_id);
            auto_collectable && already_collected
        });

    if cancel_trade {
        return Err("You already possess knowledge of all traded rewards".into());
    }

    // Remove requested items from the player inventory
    InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, entity_id, &required_item_stacks, |x| {
        get_distance(ctx, &building, deployable_location, x)
    })?;

    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        entity_id,
        &offered_item_stacks,
        |x| get_distance(ctx, &building, deployable_location, x),
        false,
        || vec![coordinates],
        false,
    )?;

    // We assume barter stalls will only have 1 inventory. If in the future we have barter stalls with crafting inventory, we will need
    // to pass the inventory_index or make sure the first function is always the barter inventory.
    if let (Some(item_index), Some(cargo_index)) = (item_index, cargo_index) {
        if let Some(item_inventory) = InventoryState::get_by_owner_with_index(ctx, shop_entity_id, item_index as i32) {
            let mut item_inventory = item_inventory;

            let mut removed_items: Vec<ItemStack> = trade_order
                .offer_items
                .iter()
                .filter(|i| i.item_type == ItemType::Item)
                .map(|i| i.clone())
                .collect();
            if claim.is_none() {
                // Everything is removed from the stall
                if !item_inventory.remove(&removed_items) {
                    return Err("Stall lacks goods or funds".into());
                }
            } else {
                // Remove offered items
                let removed_coins = extract_coins(&mut removed_items);

                if !item_inventory.remove(&removed_items) {
                    return Err("Stall lacks goods".into());
                }

                // Coins are removed first from storage then from the town treasury
                if removed_coins > 0 {
                    let barter_coins = item_inventory.coins();
                    let mut removed_from_treasury = 0;

                    if barter_coins > 0 {
                        let mut coin_item = vec![ItemStack::hex_coins(removed_coins as i32)];
                        if barter_coins >= removed_coins {
                            let _ = item_inventory.remove(&coin_item);
                        } else {
                            // pay partly from the treasury
                            // Use removed_coins directly for clarity; coin_item[0].quantity is the same value here.
                            removed_from_treasury = (removed_coins - barter_coins) as u32;
                            // and partly from the storage
                            coin_item[0].quantity = barter_coins;
                            let _ = item_inventory.remove(&coin_item);
                        }
                    } else {
                        removed_from_treasury = removed_coins as u32;
                    }

                    if removed_from_treasury > 0 {
                        let mut claim_local = claim.as_ref().unwrap().local_state(ctx);
                        // Only check the portion that must come from the treasury.
                        // Some coins may have already been removed from the stall inventory.
                        if claim_local.treasury < removed_from_treasury {
                            return Err("Vendor does not have enough funds for this transaction".into());
                        }
                        // pay only the shortfall from the treasury
                        claim_local.treasury -= removed_from_treasury;
                        ctx.db.claim_local_state().entity_id().update(claim_local);
                    }
                }
            }

            let mut gained_items: Vec<ItemStack> = trade_order
                .required_items
                .iter()
                .filter(|i| i.item_type == ItemType::Item)
                .map(|i| i.clone())
                .collect();
            if claim.is_none() {
                // Everything is added in the stall
                if !item_inventory.add_multiple(ctx, &gained_items) {
                    return Err("Stall is full".into());
                }
            } else {
                let gained_coins = extract_coins(&mut gained_items) as u32;

                // Goods are added in the stall
                if !item_inventory.add_multiple(ctx, &gained_items) {
                    return Err("Stall cannot hold more items".into());
                }

                // Coins are added to the town treasury
                if gained_coins > 0 {
                    let mut claim_local = claim.as_ref().unwrap().local_state(ctx);
                    claim_local.treasury += gained_coins;
                    ctx.db.claim_local_state().entity_id().update(claim_local);
                }
            }

            // Note / TODO: barter stalls should have a single inventory featuring both cargo and items.

            ctx.db.inventory_state().entity_id().update(item_inventory);
        }

        if let Some(cargo_inventory) = InventoryState::get_by_owner_with_index(ctx, shop_entity_id, cargo_index as i32) {
            let mut cargo_inventory = cargo_inventory;

            // Remove offered cargos
            let cargo_itemstacks: Vec<ItemStack> = trade_order
                .offer_items
                .iter()
                .filter(|i| i.item_type == ItemType::Cargo)
                .map(|i| i.clone())
                .collect();
            if !cargo_inventory.remove(&cargo_itemstacks) {
                return Err("Building inventory lacks goods".into());
            }
            // Add requested cargos
            let cargo_itemstacks: Vec<ItemStack> = trade_order
                .required_items
                .iter()
                .filter(|i| i.item_type == ItemType::Cargo)
                .map(|i| i.clone())
                .collect();
            if !cargo_inventory.add_multiple(ctx, &cargo_itemstacks) {
                return Err("Building stockpile is full".into());
            }
            ctx.db.inventory_state().entity_id().update(cargo_inventory);
        }
    } else if ctx.db.npc_state().building_entity_id().filter(shop_entity_id).next().is_none() {
        return Err("The offered items are not available".into());
    }

    if trade_order.remaining_stock != i32::MAX {
        trade_order.remaining_stock = trade_order.remaining_stock - 1;
        ctx.db.trade_order_state().entity_id().update(trade_order);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, entity_id);

    Ok(())
}

fn extract_coins(item_stacks: &mut Vec<ItemStack>) -> i32 {
    if let Some(n) = item_stacks
        .iter()
        .position(|i| i.item_type == ItemType::Item && i.item_id == TradeOrderState::MARKET_MODE_CURRENCY_ID)
    {
        let amount = item_stacks[n].quantity;
        item_stacks.remove(n);
        return amount;
    }
    0
}

fn get_distance(
    ctx: &ReducerContext,
    building_state: &Option<BuildingState>,
    deployable_location: Option<SmallHexTile>,
    coordinates: SmallHexTile,
) -> i32 {
    if let Some(building_state) = building_state {
        return building_state.distance_to(ctx, &coordinates);
    }

    if let Some(deployable_location) = deployable_location {
        return deployable_location.distance_to(coordinates);
    }

    i32::MAX
}
