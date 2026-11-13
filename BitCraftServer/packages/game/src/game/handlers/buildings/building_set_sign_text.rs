use bitcraft_macro::feature_gate;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    building_desc,
    game::{game_state, permission_helper, reducer_helpers::user_text_input_helpers::is_user_text_input_valid},
    messages::{action_request::BuildingSetSignTextRequest, components::*},
    unwrap_or_err, BuildingCategory,
};

#[spacetimedb::reducer]
#[feature_gate("build")]
pub fn building_set_sign_text(ctx: &ReducerContext, request: BuildingSetSignTextRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, request.building_entity_id, request.text)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, building_entity_id: u64, text: String) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    UserModerationState::validate_chat_privileges(ctx, &ctx.sender(), "Your naming privileges have been suspended")?;

    let building_state = unwrap_or_err!(ctx.db.building_state().entity_id().find(building_entity_id), "No such building.");
    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building_state.building_description_id),
        "Invalid building description"
    );

    if !building_desc.has_category(ctx, BuildingCategory::Sign) {
        return Err("Can't set sign text for this building.".into());
    }

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    if !permission_helper::can_interact_with_building(ctx, &building_state, actor_id, ClaimPermission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    if is_user_text_input_valid(&text, 140, true).is_err() {
        return Err("Failed to set sign text".into());
    }

    if let Some(mut player_note_state) = ctx.db.player_note_state().entity_id().find(building_entity_id) {
        player_note_state.text = text;
        ctx.db.player_note_state().entity_id().update(player_note_state);

        return Ok(());
    }

    let player_note_state = PlayerNoteState {
        entity_id: building_entity_id,
        text,
    };

    if ctx.db.player_note_state().try_insert(player_note_state).is_err() {
        log::error!("Failed to insert PlayerNoteState");
    }

    Ok(())
}
