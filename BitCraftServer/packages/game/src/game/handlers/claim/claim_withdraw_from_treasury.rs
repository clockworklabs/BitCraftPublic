use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{
        action_request::PlayerClaimWithdrawFromTreasuryRequest,
        components::*,
        events::{claim_treasury_event, ClaimTreasuryChangeReason, ClaimTreasuryEvent},
        game_util::ItemStack,
    },
    unwrap_or_err, InventoryState,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn claim_withdraw_from_treasury(ctx: &ReducerContext, request: PlayerClaimWithdrawFromTreasuryRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(&request.claim_entity_id), "No such claim.");

    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Only the owner and co-owners can withdraw from the treasury.".into());
    }

    let mut claim_local = claim.local_state(ctx);
    if request.amount > claim_local.treasury {
        return Err("Not enough funds in the treasury".into());
    }

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
    if !inventory.add(ctx, ItemStack::hex_coins(request.amount as i32)) {
        return Err("Not enough room in your inventory".into());
    }
    ctx.db.inventory_state().entity_id().update(inventory);

    claim_local.treasury -= request.amount;
    let treasury_after = claim_local.treasury;
    ctx.db.claim_local_state().entity_id().update(claim_local);

    ctx.db.claim_treasury_event().insert(ClaimTreasuryEvent {
        claim_entity_id: request.claim_entity_id,
        actor_entity_id: actor_id,
        reason: ClaimTreasuryChangeReason::Withdraw,
        amount: request.amount,
        treasury_after,
        timestamp: ctx.timestamp,
    });

    Ok(())
}