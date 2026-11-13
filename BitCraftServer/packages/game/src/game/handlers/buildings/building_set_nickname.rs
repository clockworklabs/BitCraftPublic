use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::{
    building_desc,
    game::{game_state, permission_helper, reducer_helpers::user_text_input_helpers::is_user_text_input_valid},
    messages::{action_request::PlayerBuildingSetNicknameRequest, components::*},
    unwrap_or_err, BuildingCategory,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate("build")]
pub fn building_set_nickname(ctx: &ReducerContext, request: PlayerBuildingSetNicknameRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    reduce(ctx, actor_id, request.building_entity_id, request.nickname)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, building_entity_id: u64, nickname: String) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    UserModerationState::validate_chat_privileges(ctx, &ctx.sender(), "Your naming privileges have been suspended")?;

    let building_state = unwrap_or_err!(ctx.db.building_state().entity_id().find(building_entity_id), "No such building.");

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    if !permission_helper::can_interact_with_building(ctx, &building_state, actor_id, ClaimPermission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    let building_desc = unwrap_or_err!(
        ctx.db.building_desc().id().find(building_state.building_description_id),
        "Invalid building description"
    );
    if building_desc.has_category(ctx, BuildingCategory::Portal) {
        return Err("Can't set a nickname for this building.".into());
    }

    if is_user_text_input_valid(&nickname, 35, true).is_err() {
        return Err("Failed to set Building nickname".into());
    }

    BuildingNicknameState::set_nickname(ctx, building_entity_id, nickname);

    Ok(())
}
