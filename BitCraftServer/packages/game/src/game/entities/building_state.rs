use spacetimedb::{ReducerContext, Table};

pub use crate::game::coordinates::*;
use crate::game::game_state;
use crate::game::reducer_helpers::building_helpers;
use crate::messages::components::{
    bank_state, building_nickname_state, claim_local_state, claim_tech_state, waystone_state, BankState, BuildingNicknameState,
    DimensionNetworkState, MarketplaceState, RentState, WaystoneState,
};
pub use crate::messages::components::{BuildingState, InventoryState};
use crate::messages::inter_module::EmpireCreateBuildingMsg;
use crate::messages::static_data::{construction_recipe_desc, BuildingCategory, BuildingDesc};
use crate::messages::util::SmallHexTileMessage;
use crate::{
    building_desc, building_state, claim_state, dimension_network_state, inter_module, inventory_state, marketplace_state, rent_state,
    unwrap_or_err, FootprintType,
};

impl BuildingState {
    pub fn distance_to(&self, ctx: &ReducerContext, coordinates: &SmallHexTile) -> i32 {
        coordinates.distance_to_footprint(self.footprint(ctx, game_state::game_state_filters::coordinates(ctx, self.entity_id)))
    }

    pub fn footprint(&self, ctx: &ReducerContext, center: SmallHexTile) -> Vec<(SmallHexTile, FootprintType)> {
        if let Some(building_desc) = ctx.db.building_desc().id().find(&self.building_description_id) {
            return building_desc.get_footprint(&center, self.direction_index);
        }

        Vec::new()
    }

    pub fn update_inventories(&mut self, ctx: &ReducerContext, description: &BuildingDesc) -> bool {
        let mut inventory_index = -1;

        // Update storage and crafting inventories from the upgraded building functions
        for f in &description.functions {
            if f.storage_slots > 0 || f.cargo_slots > 0 {
                //Banks will create an inventory on demand for each player
                if f.has_category(ctx, BuildingCategory::Bank) {
                    continue;
                }

                let num_pockets = f.storage_slots + f.cargo_slots;
                // A vast majority of the buildings will only have 1 inventory
                inventory_index = inventory_index + 1;
                let inventory = InventoryState::get_by_owner_with_index(ctx, self.entity_id, inventory_index);

                if inventory.is_some() {
                    let mut inventory = inventory.unwrap();
                    let previous_pockets = inventory.pockets.len() as i32;
                    if num_pockets > previous_pockets {
                        // Note: If we still can upgrade buildings and inventories, we will need to do something about cargo indexes
                        inventory.add_pockets(num_pockets - previous_pockets, f.item_slot_size);
                        ctx.db.inventory_state().entity_id().update(inventory);
                    }
                } else {
                    let cargo_index = f.storage_slots;
                    // new inventory for new feature
                    if !InventoryState::new_with_index(
                        ctx,
                        num_pockets,
                        f.item_slot_size,
                        f.cargo_slot_size,
                        inventory_index,
                        cargo_index,
                        self.entity_id,
                        0,
                        None,
                    ) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn claim(ctx: &ReducerContext, building_entity_id: u64, claim_entity_id: u64) {
        if claim_entity_id == 0 {
            return;
        }
        if let Some(building) = ctx.db.building_state().entity_id().find(&building_entity_id) {
            if building.claim_entity_id == 0 {
                // Update claim decay
                let building_desc = ctx.db.building_desc().id().find(&building.building_description_id).unwrap();
                let building_maintenance = building_desc.maintenance;
                if building_maintenance != 0.0 {
                    let claim = ctx.db.claim_state().entity_id().find(&claim_entity_id).unwrap();
                    let mut claim_local = claim.local_state(ctx);
                    claim_local.building_maintenance += building_maintenance;
                    ctx.db.claim_local_state().entity_id().update(claim_local);
                }

                // if building is a portal, also claim its interior network
                // DAB Note: might need to make it recursive to claim sub-networks
                if building_desc.has_category(ctx, BuildingCategory::Portal) {
                    if let Some(mut dimension_network_desc_state) = ctx.db.dimension_network_state().building_id().find(&building_entity_id)
                    {
                        dimension_network_desc_state.claim_entity_id = claim_entity_id;
                        ctx.db.dimension_network_state().entity_id().update(dimension_network_desc_state);
                    }
                }

                let mut building = building.clone();
                building.claim_entity_id = claim_entity_id;
                ctx.db.building_state().entity_id().update(building);
            }
        }
        if let Some(mut waystone) = ctx.db.waystone_state().building_entity_id().find(&building_entity_id) {
            waystone.claim_entity_id = claim_entity_id;
            ctx.db.waystone_state().building_entity_id().update(waystone);
        }
        if let Some(mut bank) = ctx.db.bank_state().building_entity_id().find(&building_entity_id) {
            bank.claim_entity_id = claim_entity_id;
            ctx.db.bank_state().building_entity_id().update(bank);
        }
        if let Some(mut marketplace) = ctx.db.marketplace_state().building_entity_id().find(&building_entity_id) {
            marketplace.claim_entity_id = claim_entity_id;
            ctx.db.marketplace_state().building_entity_id().update(marketplace);
        }
    }

    pub fn unclaim(ctx: &ReducerContext, building_entity_id: u64, from_destructor: bool) {
        if let Some(building) = ctx.db.building_state().entity_id().find(&building_entity_id) {
            building.unclaim_self(ctx, true, from_destructor);
        }
    }

    pub fn unclaim_self(self, ctx: &ReducerContext, update_claim_maintenance: bool, from_destructor: bool) {
        let building_entity_id = self.entity_id;
        let building_desc = ctx.db.building_desc().id().find(self.building_description_id).unwrap();

        if self.claim_entity_id != 0 {
            if update_claim_maintenance {
                // Update claim decay
                let building_maintenance = building_desc.maintenance;
                if building_maintenance != 0.0 {
                    let claim = ctx.db.claim_state().entity_id().find(&self.claim_entity_id).unwrap();
                    let mut claim_local = claim.local_state(ctx);
                    claim_local.building_maintenance -= building_maintenance;
                    ctx.db.claim_local_state().entity_id().update(claim_local);
                }
            }

            let mut building = self;
            building.claim_entity_id = 0;
            ctx.db.building_state().entity_id().update(building);
        }

        if from_destructor {
            return;
        }

        if let Some(mut waystone) = ctx.db.waystone_state().building_entity_id().find(building_entity_id) {
            waystone.claim_entity_id = 0;
            ctx.db.waystone_state().building_entity_id().update(waystone);
        }
        // Shouldn't be needed since banks SHOULD have the destroy_on_unclaim flag; still leaving this code in case it's forgotten as the alternative would be worse.
        if let Some(mut bank) = ctx.db.bank_state().building_entity_id().find(&building_entity_id) {
            bank.claim_entity_id = 0;
            ctx.db.bank_state().building_entity_id().update(bank);
        }
        // Shouldn't be needed since marketplaces SHOULD have the destroy_on_unclaim flag; still leaving this code in case it's forgotten as the alternative would be worse.
        if let Some(mut marketplace) = ctx.db.marketplace_state().building_entity_id().find(&building_entity_id) {
            marketplace.claim_entity_id = 0;
            ctx.db.marketplace_state().building_entity_id().update(marketplace);
        }
        if building_desc.destroy_on_unclaim {
            building_helpers::delete_building(ctx, 0, building_entity_id, None, false, false);
        }
    }

    pub fn create_rental(
        ctx: &ReducerContext,
        building_entity_id: u64,
        building_desc: &BuildingDesc,
        dimension: u32,
    ) -> Result<(), String> {
        if building_desc.has_category(ctx, BuildingCategory::RentTerminal) {
            // Flag that interior as rented/rentable
            if let Some(dimension_description_network) = DimensionNetworkState::get(ctx, dimension) {
                if dimension_description_network.claim_entity_id == 0 {
                    // Interior was unclaimed before the start and end of the rent terminal construction
                    return Err("Can't complete rent terminal because this interior is not claimed".into());
                }
                // Create the empty rent for that interior
                let new_rent = RentState {
                    entity_id: building_entity_id,
                    dimension_network_id: dimension_description_network.entity_id,
                    claim_entity_id: dimension_description_network.claim_entity_id,
                    white_list: Vec::new(),
                    daily_rent: 0,
                    paid_rent: 0,
                    active: false,
                    defaulted: false,
                    eviction_timestamp: None,
                };

                ctx.db.rent_state().try_insert(new_rent)?;

                let mut dimension_description_network = dimension_description_network.clone();
                dimension_description_network.rent_entity_id = building_entity_id;
                ctx.db.dimension_network_state().entity_id().update(dimension_description_network);
            }
        }
        Ok(())
    }

    pub fn create_empire_building(
        ctx: &ReducerContext,
        building_entity_id: u64,
        building_desc: &BuildingDesc,
        player_entity_id: u64,
        location: SmallHexTile,
        construction_recipe_id: Option<i32>,
    ) {
        if building_desc.has_category(ctx, BuildingCategory::EmpireFoundry) {
            inter_module::send_inter_module_message(
                ctx,
                crate::messages::inter_module::MessageContentsV5::EmpireCreateBuilding(EmpireCreateBuildingMsg {
                    player_entity_id,
                    building_entity_id,
                    location: location.into(),
                    building_desc_id: building_desc.id,
                    construction_recipe_id,
                }),
                inter_module::InterModuleDestination::Global,
            );
        }
    }

    pub fn create_waystone(
        ctx: &ReducerContext,
        building_entity_id: u64,
        claim_entity_id: u64,
        building_desc: &BuildingDesc,
        coordinates: SmallHexTileMessage,
    ) {
        if building_desc.has_category(ctx, BuildingCategory::Waystone) {
            ctx.db.waystone_state().insert(WaystoneState {
                building_entity_id: building_entity_id,
                claim_entity_id: claim_entity_id,
                coordinates: coordinates,
            });
        }
    }

    pub fn create_bank(
        ctx: &ReducerContext,
        building_entity_id: u64,
        claim_entity_id: u64,
        building_desc: &BuildingDesc,
        coordinates: SmallHexTileMessage,
    ) -> Result<(), String> {
        if building_desc.has_category(ctx, BuildingCategory::Bank) {
            if ctx.db.bank_state().claim_entity_id().filter(claim_entity_id).next().is_some() {
                return Err("Claims can only have one bank".into());
            }
            ctx.db.bank_state().insert(BankState {
                building_entity_id: building_entity_id,
                claim_entity_id: claim_entity_id,
                coordinates: coordinates,
            });
        }
        Ok(())
    }

    pub fn create_marketplace(
        ctx: &ReducerContext,
        building_entity_id: u64,
        claim_entity_id: u64,
        building_desc: &BuildingDesc,
        coordinates: SmallHexTileMessage,
    ) -> Result<(), String> {
        if building_desc.has_category(ctx, BuildingCategory::TownMarket) {
            if ctx
                .db
                .marketplace_state()
                .claim_entity_id()
                .filter(claim_entity_id)
                .next()
                .is_some()
            {
                return Err("Claims can only have one market".into());
            }
            ctx.db.marketplace_state().insert(MarketplaceState {
                building_entity_id: building_entity_id,
                claim_entity_id: claim_entity_id,
                coordinates: coordinates,
            });
        }
        Ok(())
    }

    pub fn ensure_claim_tech(&self, ctx: &ReducerContext) -> Result<(), String> {
        //To allow crafting in Ruined Towns
        if let Some(claim) = ctx.db.claim_state().entity_id().find(&self.claim_entity_id) {
            if claim.neutral {
                return Ok(());
            }
        }

        if let Some(recipe) = ctx
            .db
            .construction_recipe_desc()
            .building_description_id()
            .filter(self.building_description_id)
            .next()
        {
            if recipe.required_claim_tech_ids.len() != 0 {
                let claim_tech = unwrap_or_err!(
                    ctx.db.claim_tech_state().entity_id().find(&self.claim_entity_id),
                    "This claim is missing its tech tree"
                );
                for required_claim_tech_id in &recipe.required_claim_tech_ids {
                    // Error messages here should not be accessible to non-cheating players as those recipes are hidden if not unlocked by claim, so no need to be too specific about the details.
                    if !claim_tech.has_unlocked_tech(*required_claim_tech_id) {
                        return Err("Missing claim upgrades".into());
                    }
                }
            }
        }

        Ok(())
    }
}

impl BuildingNicknameState {
    pub fn set_nickname(ctx: &ReducerContext, building_entity_id: u64, nickname: String) {
        if let Some(mut nickname_state) = ctx.db.building_nickname_state().entity_id().find(&building_entity_id) {
            if nickname_state.nickname != nickname {
                nickname_state.nickname = nickname;
                BuildingNicknameState::update_shared(ctx, nickname_state, crate::inter_module::InterModuleDestination::Global);
            }
        } else {
            BuildingNicknameState::insert_shared(
                ctx,
                BuildingNicknameState {
                    entity_id: building_entity_id,
                    nickname,
                },
                crate::inter_module::InterModuleDestination::Global,
            );
        }
    }
}
