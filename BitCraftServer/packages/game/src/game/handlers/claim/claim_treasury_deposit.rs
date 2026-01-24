use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self, game_state_filters},
    messages::{action_request::PlayerClaimDepositToTreasuryRequest, components::*, game_util::ItemStack},
    unwrap_or_err, InventoryState,
};

#[spacetimedb::reducer]
pub fn claim_treasury_deposit(ctx: &ReducerContext, request: PlayerClaimDepositToTreasuryRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    if request.amount == 0 {
        return Err("Cannot deposit nothing.".into());
    }

    let claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(&request.claim_entity_id), "No such claim.");

    if !claim.has_co_owner_permissions(ctx, actor_id) {
        return Err("Only the owner and co-owners can deposit into the treasury.".into());
    }

    let amount = i32::try_from(request.amount)
    .map_err(|_| "Cannot deposit such a large amount.")?;

    let item_stacks = vec![ItemStack::hex_coins(amount)];
    let coord = game_state_filters::coordinates_float(ctx, actor_id).parent_small_tile();
    InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &item_stacks, |c| c.distance_to(coord))?;

    let mut claim_local = claim.local_state(ctx);
    claim_local.treasury += request.amount;
    ctx.db.claim_local_state().entity_id().update(claim_local);

    Ok(())
}
