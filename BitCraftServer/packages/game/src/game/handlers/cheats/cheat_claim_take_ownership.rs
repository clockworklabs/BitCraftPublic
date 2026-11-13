use crate::game::game_state;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::components::ClaimState;
use crate::{claim_state, player_username_state, unwrap_or_err};

use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext};

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn cheat_claim_take_ownership(ctx: &ReducerContext, claim_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatClaimTakeOwnership) {
        return Err("Unauthorized.".into());
    }

    let actor_id = game_state::actor_id(ctx, false)?;
    let mut claim = unwrap_or_err!(ctx.db.claim_state().entity_id().find(claim_entity_id), "Unknown Claim");

    if claim.owner_player_entity_id == actor_id {
        return Err("You're already the owner of this claim".into());
    }

    let player_user_name = match ctx.db.player_username_state().entity_id().find(claim.owner_player_entity_id) {
        Some(value) => value.username,
        None => "none".into(),
    };

    log::info!(
        "Cheat taking ownership of claim: {} ({}) current owner: {} ({})",
        claim.name,
        claim_entity_id,
        player_user_name,
        claim.owner_player_entity_id
    );

    match claim.get_member(ctx, actor_id) {
        Some(value) => value.set_permissions(ctx, true, true, true, true),
        None => claim.add_member(ctx, actor_id, true, true, true, true)?,
    }

    claim.owner_player_entity_id = actor_id;
    ClaimState::update_shared(ctx, claim, crate::inter_module::InterModuleDestination::Global);

    Ok(())
}
