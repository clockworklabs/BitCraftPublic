use spacetimedb::{log, ReducerContext, Table};

use crate::{
    building_desc,
    game::{game_state::game_state_filters, handlers::authentication::has_role},
    messages::{authentication::Role, components::*},
    BuildingCategory, LostItemsState,
};

#[spacetimedb::reducer]
pub fn migrate_regional_banks(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }
    let mut banks = 0;
    let mut recovered = 0;
    for building in ctx
        .db
        .building_state()
        .iter()
        .filter(|building| building.building_description_id == 1615467546)
    {
        banks += 1;
        let location = game_state_filters::coordinates_try_get(ctx, building.entity_id)?;
        if ctx.db.regional_bank_state().building_entity_id().find(building.entity_id).is_none() {
            let bank = ctx
                .db
                .bank_state()
                .building_entity_id()
                .find(building.entity_id)
                .unwrap_or(BankState {
                    building_entity_id: building.entity_id,
                    claim_entity_id: building.claim_entity_id,
                    coordinates: location.into(),
                });
            ctx.db.regional_bank_state().insert(bank);
        }
        ctx.db.bank_state().building_entity_id().delete(building.entity_id);
        let inventories: Vec<_> = ctx.db.inventory_state().owner_entity_id().filter(building.entity_id).collect();
        for inventory in &inventories {
            if inventory.player_owner_entity_id == 0 || inventory.player_owner_entity_id == building.entity_id {
                return Err(format!("Invalid personal inventory owner on bank {{0}}|~{}", building.entity_id));
            }
        }
        if !inventories.is_empty() {
            let desc = ctx
                .db
                .building_desc()
                .id()
                .find(building.building_description_id)
                .ok_or("Missing bank building description")?;
            if !desc.has_category(ctx, BuildingCategory::Bank) {
                return Err("Run the regional bank migration before changing the bank's static data".into());
            }
            LostItemsState::generate_lost_items_for_building(ctx, &building, &desc);
        }
        for inventory in inventories {
            ctx.db.inventory_state().entity_id().delete(inventory.entity_id);
            recovered += 1;
        }
    }
    log::info!("Regional banks: processed {} banks, recovered {} inventories", banks, recovered);
    Ok(())
}
