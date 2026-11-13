use bitcraft_macro::feature_gate;
use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::messages::action_request::PlayerConvertCollectibleToDeedRequest;
use crate::messages::components::*;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::static_data::premium_item_desc;
use crate::{collectible_desc, deployable_desc, unwrap_or_err, CollectibleDesc, CollectibleType, DeployableDesc};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn convert_collectible_to_deed(ctx: &ReducerContext, request: PlayerConvertCollectibleToDeedRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let mut vault: VaultState = unwrap_or_err!(ctx.db.vault_state().entity_id().find(actor_id), "Vault not initialized");

    if request.vault_index < 0 || request.vault_index >= vault.collectibles.len() as i32 {
        return Err("Invalid vault index".into());
    }

    let collectible_desc: CollectibleDesc = unwrap_or_err!(
        ctx.db
            .collectible_desc()
            .id()
            .find(vault.collectibles[request.vault_index as usize].id),
        "Invalid collectible id"
    );

    if collectible_desc.locked {
        return Err("Collectible cannot be converted into deed".into());
    }

    if collectible_desc.item_deed_id == -1 {
        return Err("Collectible cannot be converted into deed".into());
    }

    let is_premium_item = ctx
        .db
        .premium_item_desc()
        .iter()
        .any(|prem| prem.collectible_ids.contains(&collectible_desc.id));
    let vault_count = vault.collectibles[request.vault_index as usize].count;
    let stack = ItemStack::new(
        ctx,
        collectible_desc.item_deed_id,
        ItemType::Item,
        if is_premium_item { vault_count } else { 1 },
    );

    let mut discovery = Discovery::new(actor_id);

    if !InventoryState::add_and_discover(ctx, actor_id, &mut discovery, stack, false) {
        return Err("Not enough inventory space!".into());
    }

    discovery.commit(ctx);

    if vault_count == 1 || is_premium_item {
        if collectible_desc.collectible_type == CollectibleType::Deployable {
            let mut player_prefs: PlayerPrefsState =
                unwrap_or_err!(ctx.db.player_prefs_state().entity_id().find(actor_id), "Unknown PlayerPrefs");

            if player_prefs.default_deployable_collectible_id == collectible_desc.id {
                player_prefs.default_deployable_collectible_id = 0;
                ctx.db.player_prefs_state().entity_id().update(player_prefs);
            }

            let deployable_desc: DeployableDesc = unwrap_or_err!(
                ctx.db.deployable_desc().deploy_from_collectible_id().find(collectible_desc.id),
                "Unkown DeployableDesc"
            );

            if let Some(deployable) = ctx
                .db
                .deployable_state_v2()
                .owner_id()
                .filter(&actor_id)
                .find(|x| x.deployable_description_id == deployable_desc.id)
            {
                if ctx.db.mobile_entity_state().entity_id().find(deployable.entity_id).is_some() {
                    return Err(format!("Recover your {{0}} first|~{}", deployable_desc.name));
                }
                ctx.db.deployable_state_v2().entity_id().delete(deployable.entity_id);
                ctx.db
                    .deployable_collectible_state()
                    .deployable_entity_id()
                    .delete(deployable.entity_id);
                ctx.db.trade_order_state().shop_entity_id().delete(deployable.entity_id);
            }
        }
        vault.collectibles.remove(request.vault_index as usize);
    } else {
        vault.collectibles[request.vault_index as usize].count -= 1;
    }
    ctx.db.vault_state().entity_id().update(vault);

    Ok(())
}
