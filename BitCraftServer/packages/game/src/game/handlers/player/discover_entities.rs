use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    crafting_recipe_desc,
    game::{
        discovery::Discovery,
        entities::building_state::{BuildingState, InventoryState},
        game_state::{self, game_state_filters},
    },
    messages::{action_request::PlayerDiscoverEntitiesRequest, components::*, game_util::ItemType},
    parameters_desc,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_entities(ctx: &ReducerContext, request: PlayerDiscoverEntitiesRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);
    let discovery_range = ctx.db.parameters_desc().version().find(0).unwrap().discovery_range;

    let mut discovery = Discovery::new(actor_id);

    for entity_id in &request.discovered_entities_id {
        if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
            if location.coordinates().distance_to(player_coord) > discovery_range {
                // to far to discover
                continue;
            }
        }
        if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
            if location.coordinates().distance_to(player_coord) > discovery_range {
                // to far to discover
                continue;
            }
        }
        if let Some(building) = ctx.db.building_state().entity_id().find(entity_id) {
            discover_building(ctx, &mut discovery, &building);
        }
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(entity_id) {
            discovery.discover_enemy(ctx, enemy.enemy_type as i32);
        }
        if let Some(npc) = ctx.db.npc_state().entity_id().find(entity_id) {
            discovery.discover_npc(ctx, npc.npc_type as i32);
        }
        if let Some(deposit) = ctx.db.resource_state().entity_id().find(entity_id) {
            discovery.discover_resource(ctx, deposit.resource_id);
        }
        if let Some(deployable) = ctx.db.deployable_state_v2().entity_id().find(entity_id) {
            discovery.discover_deployable(ctx, deployable.deployable_description_id);
        }
        if let Some(trade_order) = ctx.db.trade_order_state().entity_id().find(entity_id) {
            let coord = game_state_filters::coordinates_any(ctx, trade_order.shop_entity_id);
            if coord.distance_to(player_coord) > discovery_range * 3 {
                // you can interact with the npc far from the building center. Let's triple the distance.
                // to far to discover
                continue;
            }
            discover_trade_order(ctx, &mut discovery, &trade_order);
        }
        // loot chests and dropped_inventories have the same entity_id as their inventory
        if let Some(inventory) = ctx.db.inventory_state().entity_id().find(entity_id) {
            let owner_entity_id = inventory.owner_entity_id;
            if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(owner_entity_id) {
                if location.coordinates().distance_to(player_coord) > discovery_range {
                    // to far to discover
                    continue;
                }
            }
            if let Some(location) = ctx.db.location_state().entity_id().find(owner_entity_id) {
                if location.coordinates().distance_to(player_coord) > discovery_range {
                    // to far to discover
                    continue;
                }
            }
            discover_inventory(ctx, &mut discovery, &inventory);
        }
        if let Some(trade) = ctx.db.trade_session_state().entity_id().find(entity_id) {
            for pocket in &trade.acceptor_offer {
                discovery.discover_item_and_item_list(ctx, pocket.contents.item_id);
            }
            for pocket in &trade.initiator_offer {
                discovery.discover_item_and_item_list(ctx, pocket.contents.item_id);
            }
        }
        if let Some(project_site) = ctx.db.project_site_state().entity_id().find(entity_id) {
            discovery.discover_construction(ctx, project_site.construction_recipe_id);
            discovery.discover_resource_placement(ctx, project_site.resource_placement_recipe_id);
        }

        if let Some(progressive_action) = ctx.db.progressive_action_state().entity_id().find(entity_id) {
            if let Some(craft_recipe) = ctx.db.crafting_recipe_desc().id().find(progressive_action.recipe_id) {
                let item_stack = craft_recipe.crafted_item_stacks[0];
                if item_stack.item_type == ItemType::Cargo {
                    discovery.discover_cargo(ctx, item_stack.item_id);
                } else {
                    discovery.discover_item_and_item_list(ctx, item_stack.item_id);
                }
            }
        }
    }

    discovery.commit(ctx);

    Ok(())
}

fn discover_trade_order(ctx: &ReducerContext, discovery: &mut Discovery, trade_order: &TradeOrderState) {
    for i in &trade_order.offer_items {
        if i.item_type == ItemType::Item {
            discovery.discover_item_and_item_list(ctx, i.item_id);
        } else {
            discovery.discover_cargo(ctx, i.item_id);
        }
    }
    for i in &trade_order.required_items {
        if i.item_type == ItemType::Item {
            discovery.discover_item_and_item_list(ctx, i.item_id);
        } else {
            discovery.discover_cargo(ctx, i.item_id);
        }
    }
}

fn discover_inventory(ctx: &ReducerContext, discovery: &mut Discovery, inventory: &InventoryState) {
    for i in 0..inventory.pockets.len() {
        let p = &inventory.pockets[i];
        if let Some(contents) = p.contents {
            if inventory.is_pocket_cargo(i) {
                discovery.discover_cargo(ctx, contents.item_id);
            } else {
                discovery.discover_item_and_item_list(ctx, contents.item_id);
            }
        }
    }
}

fn discover_building(ctx: &ReducerContext, discovery: &mut Discovery, building: &BuildingState) {
    discovery.discover_building(ctx, building.building_description_id);

    // NOTE: Since we are not seeing other player's projects for now, we won't discover anything at the moment
    /*
    // Discover all passive timers in this building (for now, queued / processing )
    for passive_craft_timer in ctx.db.passive_craft_state().building_entity_id().find(&building.entity_id)
        .filter(|p| p.status == PassiveCraftStatus::Processing || p.status == PassiveCraftStatus::Queued)
    {
        if let Some(recipe) = ctx.db.crafting_recipe_desc().id().find(&passive_craft_timer.recipe_id) {
            let item_stack = recipe.crafted_item_stacks[0];
            if recipe.crafted_item_stacks[0].item_type == ItemType::Cargo {
                discovery.discover_cargo(item_stack.item_id);
            } else {
                discovery.discover_item_and_item_list(item_stack.item_id);
            }
        }
    }
    */
}
