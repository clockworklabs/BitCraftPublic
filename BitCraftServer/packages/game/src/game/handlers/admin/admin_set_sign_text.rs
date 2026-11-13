use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        coordinates::{OffsetCoordinatesSmall, SmallHexTile},
        handlers::authentication::has_role,
    },
    messages::{
        authentication::Role,
        components::{building_state, player_note_state, LocationState, PlayerNoteState},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_set_sign_text(ctx: &ReducerContext, deployable_name: String, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let name_lower = deployable_name.to_lowercase();
    let entity_id = unwrap_or_err!(
        ctx.db
            .player_note_state()
            .iter().find(|d| d.text.to_lowercase() == name_lower),
        "Sign not found"
    )
    .entity_id;

    admin_set_sign_text_entity(ctx, entity_id, new_name)
}

#[spacetimedb::reducer]
pub fn admin_set_sign_text_coord(ctx: &ReducerContext, coord: OffsetCoordinatesSmall, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let entity_id = unwrap_or_err!(
        LocationState::select_all(ctx, &SmallHexTile::from(coord)).find(|l| ctx.db.building_state().entity_id().find(l.entity_id).is_some()),
        "No building at this location"
    )
    .entity_id;

    admin_set_sign_text_entity(ctx, entity_id, new_name)
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn admin_set_sign_text_entity(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    if ctx.db.building_state().entity_id().find(entity_id).is_none() {
        return Err("Sign doesn't exist".into());
    }

    ctx.db.player_note_state().entity_id().delete(entity_id);
    ctx.db.player_note_state().insert(PlayerNoteState { entity_id, text: new_name });

    Ok(())
}
