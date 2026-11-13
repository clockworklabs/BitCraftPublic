use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        coordinates::{OffsetCoordinatesSmall, SmallHexTile},
        handlers::authentication::has_role,
    },
    messages::{
        authentication::Role,
        components::{building_nickname_state, claim_state, BuildingNicknameState, LocationState},
    },
    unwrap_or_err,
};

use super::admin_rename_claim;

//You can call this !!!ON ALL REGIONS!!! to rename waystones

#[spacetimedb::reducer]
pub fn admin_rename_building(ctx: &ReducerContext, building_name: String, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let name_lower = building_name.to_lowercase();
    let entity_id = unwrap_or_err!(
        ctx.db
            .building_nickname_state()
            .iter().find(|d| d.nickname.to_lowercase() == name_lower),
        "Building not found"
    )
    .entity_id;

    admin_rename_building_entity(ctx, entity_id, new_name)
}

#[spacetimedb::reducer]
pub fn admin_rename_building_coord(ctx: &ReducerContext, coord: OffsetCoordinatesSmall, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let entity_id = unwrap_or_err!(
        LocationState::select_all(ctx, &SmallHexTile::from(coord)).find(|l| ctx.db.building_nickname_state().entity_id().find(l.entity_id).is_some()),
        "No nicknamed building at this location"
    )
    .entity_id;

    admin_rename_building_entity(ctx, entity_id, new_name)
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn admin_rename_building_entity(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    if ctx.db.claim_state().entity_id().find(entity_id).is_some() {
        return admin_rename_claim::reduce(ctx, entity_id, new_name);
    }
    if let Some(c) = ctx.db.claim_state().owner_building_entity_id().find(entity_id) {
        return admin_rename_claim::reduce(ctx, c.entity_id, new_name);
    }

    BuildingNicknameState::set_nickname(ctx, entity_id, new_name);

    Ok(())
}
