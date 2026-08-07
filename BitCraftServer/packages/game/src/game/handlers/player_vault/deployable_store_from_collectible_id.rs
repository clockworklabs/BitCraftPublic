use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self},
    inter_module::send_inter_module_message,
    messages::{
        components::*,
        static_data::{collectible_desc, CollectibleType},
    },
    unwrap_or_err,
};

// This reducer handles storing a deployable for an error case where the deployable is
// active in the vault but the deployable collectible state is missing or has no location.
#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_store_from_collectible_id(ctx: &ReducerContext, collectible_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    let collectible_desc = unwrap_or_err!(ctx.db.collectible_desc().id().find(&collectible_id), "Invalid collectible id");
    if collectible_desc.collectible_type != CollectibleType::Deployable {
        return Err("Collectible is not a deployable".into());
    };

    // First check that the colectible is active in the player's vault
    let mut player_vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(&actor_id), "Player vault not found for actor");
    if !player_vault.collectibles.iter().any(|c| c.id == collectible_id && c.activated) {
        return Err("Collectible not found or inactive in player vault".into());
    }

    if let Some(deployable_collectible_state) = ctx
        .db
        .deployable_collectible_state()
        .owner_entity_id()
        .filter(actor_id)
        .find(|d| d.collectible_id == collectible_id)
    {
        if deployable_collectible_state.location.is_some() {
            return Err("This reducer is only to be used in error cases".into());
        }
        // This deployable is marked as active in the vault but has no location in the collectible state.
        // We should delete the collectible state so it can be restored
        ctx.db
            .deployable_collectible_state()
            .deployable_entity_id()
            .delete(deployable_collectible_state.deployable_entity_id);
    }

    // Remove the collectible from the vault and add it back
    // This will recreate the deployable_state and deployable_collectible_state for it
    player_vault.collectibles.retain(|c| c.id != collectible_id);
    player_vault.add_collectible(ctx, collectible_id, false)?;
    ctx.db.vault_state().entity_id().update(player_vault);

    //Send inter module message to store deployable in case it was left behind somwhere else.
    //We don't know which region the deployable is on, so we just blast messages to all regions and see if one of them succeeds
    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::RecoverDeployable(crate::messages::inter_module::RecoverDeployableMsg {
            player_entity_id: actor_id,
            deployable_entity_id: 0,
            deployable_desc_id: collectible_id,
        }),
        crate::inter_module::InterModuleDestination::AllOtherRegions,
    );

    Ok(())
}
