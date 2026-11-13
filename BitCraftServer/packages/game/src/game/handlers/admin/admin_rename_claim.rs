use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::handlers::{authentication::has_role, claim::claim_rename},
    messages::{authentication::Role, components::claim_state},
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_rename_claim(ctx: &ReducerContext, claim_name: String, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let name_lower = claim_name.to_lowercase();
    let entity_id = unwrap_or_err!(
        ctx.db.claim_state().iter().find(|d| d.name.to_lowercase() == name_lower),
        "Claim not found"
    )
    .entity_id;

    admin_rename_claim_entity(ctx, entity_id, new_name)
}

#[shared_table_reducer]
#[spacetimedb::reducer]
pub fn admin_rename_claim_entity(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let entity_id = if ctx.db.claim_state().entity_id().find(entity_id).is_some() {
        entity_id
    } else {
        unwrap_or_err!(ctx.db.claim_state().owner_building_entity_id().find(entity_id), "Claim not found").entity_id
    };

    reduce(ctx, entity_id, new_name)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    claim_rename::reduce(
        ctx,
        0,
        crate::messages::action_request::PlayerClaimRenameRequest {
            claim_entity_id: entity_id,
            claim_name: new_name,
        },
    )
}
