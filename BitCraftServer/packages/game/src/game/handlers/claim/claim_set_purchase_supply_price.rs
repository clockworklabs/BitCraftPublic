use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state,
    messages::{action_request::ClaimSetPurchaseSupplyPriceRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_set_purchase_supply_price(ctx: &ReducerContext, request: ClaimSetPurchaseSupplyPriceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if request.purchase_price < 0.0 {
        return Err("Cannot purchase supplies for a negative value".into());
    }

    let building_entity_id = request.building_entity_id;
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(building_entity_id),
        "No such building to repair."
    );

    let claim = unwrap_or_err!(
        ctx.db.claim_state().entity_id().find(building.claim_entity_id),
        "No such claim."
    );

    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Only the owner or co-owners can set those values.".into());
    }

    let mut claim_local = claim.local_state(ctx);
    claim_local.supplies_purchase_price = request.purchase_price;

    ctx.db.claim_local_state().entity_id().update(claim_local);

    Ok(())
}
