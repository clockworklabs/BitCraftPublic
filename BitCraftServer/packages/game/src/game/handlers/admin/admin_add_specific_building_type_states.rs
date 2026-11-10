use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::{
            bank_state, building_state, location_state, marketplace_state, regional_bank_state, waystone_state, BankState,
            MarketplaceState, WaystoneState,
        },
        static_data::{building_desc, BuildingCategory},
    },
};

#[spacetimedb::reducer]
pub fn admin_add_specific_building_type_states(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    for building in ctx.db.building_state().iter() {
        let building_entity_id = building.entity_id;
        let claim_entity_id = building.claim_entity_id;
        if claim_entity_id > 0 {
            if let Some(desc) = ctx.db.building_desc().id().find(building.building_description_id) {
                if let Some(location) = ctx.db.location_state().entity_id().find(building_entity_id) {
                    let coordinates = location.coordinates();
                    if desc.has_category(ctx, BuildingCategory::Waystone) || desc.has_category(ctx, BuildingCategory::OneWayWaystone) {
                        if ctx.db.waystone_state().building_entity_id().find(building_entity_id).is_none() {
                            ctx.db.waystone_state().insert(WaystoneState {
                                building_entity_id: building_entity_id,
                                claim_entity_id: claim_entity_id,
                                coordinates: coordinates,
                            });
                        }
                    }
                    if desc.has_category(ctx, BuildingCategory::RegionalBank) {
                        if ctx.db.regional_bank_state().building_entity_id().find(building_entity_id).is_none() {
                            ctx.db.regional_bank_state().insert(BankState {
                                building_entity_id,
                                claim_entity_id,
                                coordinates,
                            });
                        }
                        ctx.db.bank_state().building_entity_id().delete(building_entity_id);
                    }
                    if desc.has_category(ctx, BuildingCategory::Bank) {
                        if ctx.db.bank_state().building_entity_id().find(building_entity_id).is_none() {
                            ctx.db.bank_state().insert(BankState {
                                building_entity_id,
                                claim_entity_id,
                                coordinates,
                            });
                        }
                        ctx.db.regional_bank_state().building_entity_id().delete(building_entity_id);
                    }
                    if desc.has_category(ctx, BuildingCategory::TownMarket) {
                        if ctx.db.marketplace_state().building_entity_id().find(building_entity_id).is_none() {
                            ctx.db.marketplace_state().insert(MarketplaceState {
                                building_entity_id: building_entity_id,
                                claim_entity_id: claim_entity_id,
                                coordinates: coordinates,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
