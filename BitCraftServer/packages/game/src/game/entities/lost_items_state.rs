use std::collections::HashMap;

use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        dimensions,
        game_state::{self, game_state_filters},
    },
    messages::{
        components::{
            building_state, buy_order_state, closed_listing_state, inventory_state, lost_items_state, passive_craft_state,
            progressive_action_state, sell_order_state, AlertState, ClosedListingState, DeployableStateV2, DimensionNetworkState,
            LostItemsState, PassiveCraftStatus,
        },
        game_util::ItemStack,
        static_data::{building_desc, crafting_recipe_desc, AlertType, BuildingCategory, BuildingDesc, CraftingRecipeDesc, ItemListDesc},
        util::{OffsetCoordinatesSmallMessage, SmallHexTileMessage},
    },
};

use super::{
    building_state::{BuildingState, InventoryState},
    enemy_state::SmallHexTile,
};

impl LostItemsState {
    pub fn generate_lost_items_for_building(ctx: &ReducerContext, building: &BuildingState, building_desc: &BuildingDesc) {
        let location = game_state_filters::coordinates(ctx, building.entity_id);

        // Bank
        if building_desc.has_category(ctx, BuildingCategory::Bank) {
            for inv in ctx.db.inventory_state().owner_entity_id().filter(building.entity_id) {
                let inventory_items = inv.as_item_stacks();
                Self::update(ctx, inv.player_owner_entity_id, location, inventory_items);
            }
        }

        // Last Marketplace on a claim
        if building_desc.has_category(ctx, BuildingCategory::TownMarket) {
            let claim = building.claim_entity_id;
            let town_market_building_ids: Vec<i32> = ctx
                .db
                .building_desc()
                .iter()
                .filter_map(|desc| {
                    if desc.has_category(ctx, BuildingCategory::TownMarket) {
                        Some(desc.id)
                    } else {
                        None
                    }
                })
                .collect();

            let town_markets_count_on_claim = ctx
                .db
                .building_state()
                .claim_entity_id()
                .filter(claim)
                .filter(|b| town_market_building_ids.contains(&b.building_description_id))
                .count();

            if town_markets_count_on_claim == 1 {
                Self::generate_lost_items_from_market(ctx, claim, location);
            }
        }

        // Crafting station
        if building_desc.has_category(ctx, BuildingCategory::Crafting) {
            let mut lost_items_dict: HashMap<u64, Vec<ItemStack>> = HashMap::new();
            let mut recipes_cache: HashMap<i32, CraftingRecipeDesc> = HashMap::new();

            // Recover passive crafts (completed outputs and processing/queued inputs)
            for craft in ctx.db.passive_craft_state().building_entity_id().filter(building.entity_id) {
                let recipe = recipes_cache
                    .entry(craft.recipe_id)
                    .or_insert(ctx.db.crafting_recipe_desc().id().find(craft.recipe_id).unwrap());
                match craft.status {
                    PassiveCraftStatus::Complete => {
                        // Completed, stock recipe outcome
                        lost_items_dict.entry(craft.owner_entity_id).or_default().extend(
                            recipe
                                .crafted_item_stacks
                                // output items may be item lists, resolve them before adding them to the lost inventory state
                                .iter()
                                .flat_map(|is| ItemListDesc::extract_item_stacks_from_item(ctx, *is, Some(craft.owner_entity_id))),
                        );
                    }
                    _ => {
                        // Processing or Queued, stock input items
                        lost_items_dict.entry(craft.owner_entity_id).or_default().extend(
                            recipe
                                .consumed_item_stacks
                                .iter()
                                // input items should never be item lists, put them directly into the lost items inventory
                                .map(|iis| ItemStack::new(ctx, iis.item_id, iis.item_type, iis.quantity)),
                        );
                    }
                }
            }

            // Recover active crafts (completed outputs and queued inputs)
            for progressive_action in ctx.db.progressive_action_state().building_entity_id().filter(building.entity_id) {
                let recipe = recipes_cache
                    .entry(progressive_action.recipe_id)
                    .or_insert(ctx.db.crafting_recipe_desc().id().find(progressive_action.recipe_id).unwrap());

                let completed_crafts = progressive_action.get_completed_crafts(recipe.actions_required);
                let refunded_crafts = progressive_action.get_refunded_crafts(recipe.actions_required);

                lost_items_dict
                    .entry(progressive_action.owner_entity_id)
                    .or_default()
                    .extend(
                        recipe
                            .crafted_item_stacks
                            .iter()
                            // output items may be item lists, resolve them before adding them to the lost inventory state
                            .flat_map(|is| {
                                let item_stack = ItemStack::new(ctx, is.item_id, is.item_type, is.quantity * completed_crafts);
                                ItemListDesc::extract_item_stacks_from_item(ctx, item_stack, Some(progressive_action.owner_entity_id))
                            }),
                    );

                lost_items_dict
                    .entry(progressive_action.owner_entity_id)
                    .or_default()
                    .extend(
                        recipe
                            .consumed_item_stacks
                            .iter()
                            // input items should never be item lists, put them directly into the lost items inventory
                            .map(|iis| ItemStack::new(ctx, iis.item_id, iis.item_type, iis.quantity * refunded_crafts)),
                    );
            }

            for (player_entity_id, lost_items) in lost_items_dict {
                Self::update(ctx, player_entity_id, location, lost_items);
            }
        }
    }

    pub fn generate_lost_items_from_market(ctx: &ReducerContext, claim: u64, location: SmallHexTileMessage) {
        // This is the only market on the claim and it's getting destroyed. We need to:
        // - cancel all buy and sell orders
        for order in ctx.db.buy_order_state().claim_entity_id().filter(claim) {
            order.cancel_buy_order(ctx);
        }
        for order in ctx.db.sell_order_state().claim_entity_id().filter(claim) {
            order.cancel_sell_order(ctx);
        }
        // - transfer all concluded orders into "lost and found" items
        let mut listing_dict: HashMap<u64, Vec<ClosedListingState>> = HashMap::new();

        for closed_listing in ctx.db.closed_listing_state().claim_entity_id().filter(claim) {
            let closed_listing_entity_id = closed_listing.entity_id;
            listing_dict
                .entry(closed_listing.owner_entity_id)
                .or_default()
                .push(closed_listing);
            ctx.db.closed_listing_state().entity_id().delete(closed_listing_entity_id);
        }

        for (player_entity_id, listings) in listing_dict {
            let all_player_items: Vec<ItemStack> = listings.iter().map(|l| l.item_stack).collect();
            Self::update(ctx, player_entity_id, location, all_player_items);
        }
    }

    pub fn generate_lost_items_for_deployable(ctx: &ReducerContext, deployable: &DeployableStateV2, overworld_location: SmallHexTile) {
        Self::generate_lost_items_from_inventories(ctx, deployable.entity_id, deployable.owner_id, overworld_location);
    }

    // this function is to recover items from inventories owned by world entities such as buildings or deployables
    fn generate_lost_items_from_inventories(
        ctx: &ReducerContext,
        inventory_owning_entity_id: u64, // Not necessarily the player; Bank inventories are owned by the building yet are associated to a player
        owning_player_id: u64,
        overworld_location: SmallHexTile,
    ) {
        let mut all_items = Vec::new();
        for inv in ctx.db.inventory_state().owner_entity_id().filter(inventory_owning_entity_id) {
            let mut inventory_items = inv.as_item_stacks();
            all_items.append(&mut inventory_items);
        }
        Self::update(ctx, owning_player_id, overworld_location, all_items);
    }

    fn update(ctx: &ReducerContext, owner_entity_id: u64, location: SmallHexTile, mut items: Vec<ItemStack>) {
        if items.is_empty() {
            return;
        }

        let mut offset_loc: OffsetCoordinatesSmallMessage = location.into();

        // If in an interior, use the coordinates of the portal in the overworld
        if location.dimension != dimensions::OVERWORLD {
            let mut d = location.dimension;
            while d != dimensions::OVERWORLD {
                let network = DimensionNetworkState::get(ctx, d).unwrap();
                offset_loc = game_state_filters::coordinates(ctx, network.building_id).into();
                d = offset_loc.dimension;
            }
        }

        let mut existing_inventory: Option<InventoryState> = None;

        let mut lost_item_state = ctx
            .db
            .lost_items_state()
            .owner_entity_id()
            .filter(owner_entity_id).find(|li| li.location == offset_loc);

        if lost_item_state.is_some() {
            let inventory_entity_id = lost_item_state.as_ref().unwrap().inventory_entity_id;
            existing_inventory = ctx.db.inventory_state().entity_id().find(inventory_entity_id);
            // Not sure how it happens, but the inventory might have disappeared, in which case the lost item state is no longer valid
            if existing_inventory.is_none() {
                ctx.db.lost_items_state().inventory_entity_id().delete(inventory_entity_id);
                lost_item_state = None;
            }
        }

        if lost_item_state.is_none() {
            // We create a new state and inventory for this location
            let entity_id = game_state::create_entity(ctx);
            let inventory =
                InventoryState::create_with_pockets(entity_id, 48, 600000, 600000, -1, 32, owner_entity_id, owner_entity_id, None);
            existing_inventory = Some(inventory.clone());
            ctx.db.inventory_state().insert(inventory);
            let lost_items_state = LostItemsState {
                inventory_entity_id: entity_id,
                owner_entity_id,
                location: offset_loc,
            };
            ctx.db.lost_items_state().insert(lost_items_state);
        }

        let mut inventory = existing_inventory.unwrap();
        for item_stack in items.drain(0..) {
            while !inventory.add_self(ctx, item_stack) {
                inventory.double();
            }
        }
        ctx.db.inventory_state().entity_id().update(inventory);

        _ = AlertState::new(ctx, AlertType::NewLostItems, owner_entity_id, owner_entity_id);
    }
}
