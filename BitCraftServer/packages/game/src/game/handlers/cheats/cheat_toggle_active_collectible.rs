use crate::{
    collectible_desc,
    game::handlers::cheats::cheat_type::{can_run_cheat, CheatType},
    vault_state, CollectibleType,
};
use spacetimedb::ReducerContext;

use crate::{messages::action_request::CheatToggleActiveCollectibleRequest, unwrap_or_err};

#[spacetimedb::reducer]
fn cheat_toggle_active_collectible(ctx: &ReducerContext, request: CheatToggleActiveCollectibleRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatToggleActiveCollectible) {
        return Err("Unauthorized.".into());
    }

    reduce(ctx, request.owner_entity_id, request.item_deed_id)
}

pub fn reduce(ctx: &ReducerContext, player_id: u64, collectible_id: i32) -> Result<(), String> {
    let mut vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(player_id), "Vault not initialized");
    let collectible = match ctx.db.collectible_desc().id().find(collectible_id) {
        Some(c) => c,
        None => return Err(format!("Could not find collectible with id {}", collectible_id)),
    };

    let mut exists = false;
    let mut activated = true;
    for i in 0..vault.collectibles.len() {
        // if it's already there toggle activated field
        if vault.collectibles[i].id == collectible_id {
            vault.collectibles[i].activated = !vault.collectibles[i].activated;
            activated = vault.collectibles[i].activated;
            exists = true;
        }
    }

    // if it's not there add it and set it to active
    if !exists {
        let _ = vault.add_collectible(ctx, collectible_id, false);
    }

    if collectible.collectible_type != CollectibleType::Deployable
        && collectible.collectible_type != CollectibleType::HousingFloor
        && collectible.collectible_type != CollectibleType::HousingWalls
    {
        // go through all collectibles and deactivate all others of the this type
        for i in 0..vault.collectibles.len() {
            let desc = ctx.db.collectible_desc().id().find(vault.collectibles[i].id).unwrap();
            if desc.collectible_type == collectible.collectible_type {
                vault.collectibles[i].activated = activated && vault.collectibles[i].id == collectible_id;
            }
        }
    }

    ctx.db.vault_state().entity_id().update(vault);

    Ok(())
}
