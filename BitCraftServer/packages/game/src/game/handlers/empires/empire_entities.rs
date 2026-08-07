use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{permission_helper, reducer_helpers::player_action_helpers::post_reducer_update_cargo},
    inter_module::send_inter_module_message,
    messages::{
        components::{self, *},
        empire_shared::*,
        game_util::ItemStack,
        inter_module::EmpireSiegeAddSuppliesMsg,
        static_data::*,
    },
    unwrap_or_err,
};

impl EmpireState {
    pub fn remove_crown_status(ctx: &ReducerContext, player_entity_id: u64) {
        // Remove all crown collectibles from that player's vault
        let mut vault = ctx.db.vault_state().entity_id().find(&player_entity_id).unwrap();
        for entry in ctx.db.empire_territory_desc().iter() {
            let collectible_id = entry.crown_collectible_id;
            if collectible_id != 0 {
                if let Some(index) = vault.collectibles.iter().position(|c| c.id == collectible_id) {
                    vault.collectibles.remove(index);
                }
            }
        }

        ctx.db.vault_state().entity_id().update(vault);
    }

    pub fn unequip_cloak(ctx: &ReducerContext, player_entity_id: u64) {
        // Remove or enable all cloak collectibles from that player's vault.
        // Design: There's only one cloak, and it's only when you're part of empire.
        let mut vault = ctx.db.vault_state().entity_id().find(&player_entity_id).unwrap();
        // Note: we can't index collectibles by an enum yet, therefore we need to iterate through all entries.

        for cloak_collectible in ctx
            .db
            .collectible_desc()
            .iter()
            .filter(|c| c.collectible_type == CollectibleType::ClothesCape)
        {
            if let Some(collectible) = vault.collectibles.iter_mut().find(|c| c.id == cloak_collectible.id) {
                collectible.activated = false;
            }
        }

        ctx.db.vault_state().entity_id().update(vault);
    }

    pub fn update_crown_status(ctx: &ReducerContext, empire_entity_id: u64) -> Result<(), String> {
        // Unlock or lock crown collectibles based on empire size
        let num_chunks = ctx.db.empire_chunk_state().empire_entity_id().filter(empire_entity_id).count() as u16;

        let emperor_entity_id = unwrap_or_err!(
            ctx.db
                .empire_player_data_state()
                .empire_entity_id()
                .filter(empire_entity_id)
                .filter(|data| data.rank == 0)
                .next(),
            "Emperor doesn't exist"
        )
        .entity_id;

        let mut vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(&emperor_entity_id), "Missing VaultState");
        let mut highest_crown_collectible_id = 0;
        let mut crowns: Vec<EmpireTerritoryDesc> = ctx
            .db
            .empire_territory_desc()
            .iter()
            .filter(|x| x.crown_collectible_id != 0)
            .collect();
        crowns.sort_by_key(|entry| entry.chunks);
        let previous_crown_activated = vault
            .collectibles
            .iter()
            .find(|c| crowns.iter().any(|entry| entry.crown_collectible_id == c.id))
            .map(|c| c.activated)
            .unwrap_or(true);

        for crown in crowns {
            if let Some(index) = vault.collectibles.iter().position(|c| c.id == crown.crown_collectible_id) {
                vault.collectibles.remove(index);
            }

            if num_chunks >= crown.chunks {
                highest_crown_collectible_id = crown.crown_collectible_id;
            }
        }

        if highest_crown_collectible_id != 0 {
            vault.collectibles.push(VaultCollectible {
                id: highest_crown_collectible_id,
                activated: previous_crown_activated,
                count: 1,
            });
        }

        ctx.db.vault_state().entity_id().update(vault);
        Ok(())
    }
}

impl EmpirePlayerDataState {
    pub fn has_permission_to_use_empire_building(
        ctx: &ReducerContext,
        player_entity_id: u64,
        building_entity_id: u64,
        permission: EmpirePermission,
    ) -> bool {
        let empire_entity_id = EmpireState::get_building_empire_entity_id(ctx, building_entity_id);
        if empire_entity_id == 0 {
            let building_state = ctx.db.building_state().entity_id().find(&building_entity_id).unwrap();
            // claim permission type is irrelevent here

            if !PermissionState::can_interact_with_building(ctx, player_entity_id, &building_state, Permission::Inventory) {
                return false;
            }

            return permission_helper::can_interact_with_building(
                ctx,
                &building_state,
                player_entity_id,
                components::ClaimPermission::Inventory,
            );
        }
        Self::has_permission(ctx, player_entity_id, permission)
    }
}

impl EmpireNodeSiegeState {
    pub fn validate_building(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
        // for now, make sure we only siege watch towers
        let building_state = unwrap_or_err!(
            ctx.db.building_state().entity_id().find(&building_entity_id),
            "This building does not exist"
        );
        let building_desc = unwrap_or_err!(
            ctx.db.building_desc().id().find(&building_state.building_description_id),
            "Invalid building"
        );
        if !building_desc.has_category(ctx, BuildingCategory::Watchtower) {
            return Err("You can only siege watchtowers".into());
        }
        Ok(())
    }

    pub fn consume_player_cargo(ctx: &ReducerContext, building_entity_id: u64, player_entity_id: u64) -> Result<(i32, i32), String> {
        // Remove from cart or player inventory
        let removed_supplies = vec![ItemStack::hexite_capsule()];
        let building = ctx.db.building_state().entity_id().find(building_entity_id).unwrap();
        InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, player_entity_id, &removed_supplies, |x| {
            building.distance_to(ctx, &x)
        })?;
        let hexite_capsule_id = removed_supplies[0].item_id;

        let supplies = ctx.db.empire_supplies_desc().cargo_id().find(hexite_capsule_id).unwrap().energy;

        post_reducer_update_cargo(ctx, player_entity_id);

        return Ok((supplies, hexite_capsule_id));
    }

    pub fn add_supplies(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
        proxy_empire_entity_id: Option<u64>,
    ) -> Result<(), String> {
        Self::validate_action(ctx, actor_id, building_entity_id)?;
        Self::validate_building(ctx, building_entity_id)?;
        Self::validate_add_supplies(ctx, actor_id, building_entity_id, proxy_empire_entity_id)?;
        // Add supplies to ongoing siege, either for attacker or defender. We cannot start a new siege here.

        let rank = ctx.db.empire_player_data_state().entity_id().find(&actor_id).unwrap();
        let mut siege = EmpireNodeSiegeState::get(ctx, building_entity_id, rank.empire_entity_id);

        let participating = match siege.as_ref() {
            Some(s) => s.active,
            None => false,
        };

        if !participating {
            // There is no aligned empire participating in the current active siege, but maybe the player is helping a valid empire?
            if let Some(proxy_empire_entity_id) = proxy_empire_entity_id {
                siege = EmpireNodeSiegeState::get(ctx, building_entity_id, proxy_empire_entity_id);
            }
        }

        // Remove from cart or player inventory
        let removed_supplies = vec![ItemStack::hexite_capsule()];
        let building = ctx.db.building_state().entity_id().find(building_entity_id).unwrap();
        InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &removed_supplies, |x| {
            building.distance_to(ctx, &x)
        })?;
        let hexite_capsule_id = removed_supplies[0].item_id;

        let supplies = ctx.db.empire_supplies_desc().cargo_id().find(hexite_capsule_id).unwrap().energy;

        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::EmpireSiegeAddSupplies(EmpireSiegeAddSuppliesMsg {
                siege_entity_id: siege.unwrap().entity_id,
                player_entity_id: actor_id,
                supplies,
                supply_cargo_id: hexite_capsule_id,
            }),
            crate::inter_module::InterModuleDestination::Global,
        );

        post_reducer_update_cargo(ctx, actor_id);

        Ok(())
    }
}
