use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext};

use crate::{
    game::{game_state, reducer_helpers::user_text_input_helpers::is_user_text_input_valid},
    inter_module,
    messages::{action_request::PlayerClaimRenameRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn claim_rename(ctx: &ReducerContext, request: PlayerClaimRenameRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    UserModerationState::validate_chat_privileges(ctx, &ctx.sender(), "Your naming privileges have been suspended")?;

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");
    if !claim.has_owner_permissions(actor_id) {
        return Err("Only the owner can rename the claim.".into());
    }

    if request.claim_name.to_lowercase().starts_with("claimed area (") {
        return Err("This name cannot be used".into());
    }

    reduce(ctx, actor_id, request)
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, request: PlayerClaimRenameRequest) -> Result<(), String> {
    let _ = unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.claim_entity_id), "No such claim.");

    if let Err(msg) = is_user_text_input_valid(&request.claim_name, 35, true) {
        log::info!("Failed to rename claim: {msg}");
        return Err("This name cannot be used".into());
    }

    if ctx
        .db
        .claim_lowercase_name_state()
        .name_lowercase()
        .find(request.claim_name.to_lowercase())
        .is_some()
    {
        return Err("This name is already taken".into());
    }

    inter_module::claim_set_name::send_message(ctx, actor_id, request.claim_entity_id, request.claim_name);

    Ok(())
}
