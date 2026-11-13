use spacetimedb::{ReducerContext, Table};

use crate::{
    deployable_collectible_state, deployable_state_v2,
    game::game_state,
    messages::{
        components::{VaultCollectible, VaultState},
        static_data::*,
    },
    player_prefs_state, vault_state, DeployableCollectibleState, DeployableStateV2, PlayerState,
};

impl VaultState {
    pub fn add_collectibles(ctx: &ReducerContext, actor_id: u64, collectible_ids: Vec<i32>) {
        let mut vault = ctx.db.vault_state().entity_id().find(actor_id).unwrap();
        let mut updated = false;
        for collectible_id in collectible_ids {
            match vault.add_collectible(ctx, collectible_id, false) {
                Ok(()) => updated = true,
                Err(msg) => spacetimedb::log::error!("Failed to add collectible {collectible_id} for player {actor_id}: {msg}"),
            };
        }
        if updated {
            ctx.db.vault_state().entity_id().update(vault);
        }
    }

    pub fn has_collectible(&self, id: i32) -> bool {
        self.collectibles.iter().any(|c| c.id == id)
    }

    pub fn remove_collectible_quantity(&mut self, collectible_id: i32, quantity: u32) -> u32 {
        if quantity == 0 {
            return 0;
        }

        let Some(index) = self.collectibles.iter().position(|c| c.id == collectible_id) else {
            return 0;
        };

        let owned = self.collectibles[index].count.max(0) as u32;
        let removed = owned.min(quantity);

        if removed == owned {
            self.collectibles.remove(index);
        } else {
            self.collectibles[index].count -= removed as i32;
        }

        removed
    }

    pub fn add_collectible(&mut self, ctx: &ReducerContext, collectible_id: i32, add_if_locked: bool) -> Result<(), String> {
        if let Some(collectible_desc) = ctx.db.collectible_desc().id().find(collectible_id) {
            let mut exists = false;
            for i in 0..self.collectibles.len() {
                if self.collectibles[i].id == collectible_id {
                    if collectible_desc.locked && !add_if_locked {
                        // locked collectibles can only be accounted once.
                        return Err("Already own collectible".into());
                    }
                    self.collectibles[i].count += 1;
                    exists = true;
                    break;
                }
            }
            if !exists {
                // Assign as default deployable if collecting a deployable for the first time
                self.collectibles.push(VaultCollectible {
                    id: collectible_id,
                    count: 1,
                    activated: false,
                });

                if collectible_desc.collectible_type == CollectibleType::Deployable {
                    let mut prefs = ctx.db.player_prefs_state().entity_id().find(self.entity_id).unwrap();
                    if prefs.default_deployable_collectible_id == 0 {
                        prefs.default_deployable_collectible_id = collectible_desc.id;
                        ctx.db.player_prefs_state().entity_id().update(prefs);
                    }

                    // Create location-less deployable
                    let deployable_description = ctx
                        .db
                        .deployable_desc()
                        .deploy_from_collectible_id()
                        .find(collectible_desc.id)
                        .unwrap();
                    let username = PlayerState::username_by_id(ctx, self.entity_id).unwrap();
                    let deployable = ctx
                        .db
                        .deployable_state_v2()
                        .try_insert(DeployableStateV2 {
                            entity_id: game_state::create_entity(ctx),
                            owner_id: self.entity_id,
                            claim_entity_id: 0,
                            direction: 0,
                            deployable_description_id: deployable_description.id,
                            appearance_override_id: 0,
                            nickname: format!("{}'s {}", username, deployable_description.name),
                            hidden: false,
                        })
                        .ok()
                        .unwrap();

                    let _ = ctx.db.deployable_collectible_state().try_insert(DeployableCollectibleState {
                        owner_entity_id: self.entity_id,
                        deployable_entity_id: deployable.entity_id,
                        collectible_id,
                        deployable_desc_id: deployable_description.id,
                        location: None,
                        auto_follow: false,
                    });

                    // DAB Note: We will need to remove deployable collectibles from the collectibles list and only work with DeployableCollectibleStateV2s.
                    // For now, content will limit to 1 deployable of a type per player.
                }
            }
            return Ok(());
        }
        Err("Collectible doesn't exist".into())
    }
}
