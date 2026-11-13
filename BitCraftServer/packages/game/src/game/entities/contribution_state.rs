use std::collections::{HashMap, HashSet};

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        discovery::Discovery,
        game_state::{self, game_state_filters},
    },
    messages::{
        components::{contribution_state, enemy_state, player_state, InventoryState, ThreatState},
        game_util::ItemType,
        static_data::{contribution_loot_desc, enemy_desc, item_list_desc, ItemListDesc},
    },
    ContributionState,
};

impl ContributionState {
    pub fn add_damage(ctx: &ReducerContext, player_entity_id: u64, enemy_entity_id: u64, damage: i32) {
        // We add 1000 contribution (e.g. itemlist rolls) for each 100% healthbar removed
        // Yes, this means that a boss that heals itself might yield way more than 1000 contributions this can be milked for rare items
        let enemy_state = ctx.db.enemy_state().entity_id().find(enemy_entity_id).unwrap();
        let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_state.enemy_type as i32).unwrap();
        let contribution_gain = damage as f32 / (enemy_desc.max_health as f32 / 1000.0);

        if let Some(mut contribution) = ctx
            .db
            .contribution_state()
            .player_enemy_entity_id()
            .filter((player_entity_id, enemy_entity_id))
            .next()
        {
            contribution.contribution += contribution_gain;
            ctx.db.contribution_state().entity_id().update(contribution);
            return;
        }

        ctx.db.contribution_state().insert(ContributionState {
            entity_id: game_state::create_entity(ctx),
            player_entity_id,
            enemy_entity_id,
            contribution: contribution_gain,
        });
    }

    pub fn clear(ctx: &ReducerContext, enemy_entity_id: u64) {
        ctx.db.contribution_state().enemy_entity_id().delete(enemy_entity_id);
    }

    pub fn idle_reset(ctx: &ReducerContext, enemy_entity_id: u64) {
        // Contribution resets as soon as the target is out of combat
        if ctx.db.enemy_state().entity_id().find(enemy_entity_id).is_some()
            && !ThreatState::in_combat(ctx, enemy_entity_id) {
                Self::clear(ctx, enemy_entity_id);
            }
    }

    pub fn applies(ctx: &ReducerContext, defender_entity_id: u64) -> bool {
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(defender_entity_id) {
            return ctx
                .db
                .contribution_loot_desc()
                .enemy_type_id()
                .filter(enemy.enemy_type as i32)
                .next()
                .is_some();
        }
        false
    }

    pub fn roll_all(ctx: &ReducerContext, enemy_entity_id: u64) {
        let enemy_type_id = ctx.db.enemy_state().entity_id().find(enemy_entity_id).unwrap().enemy_type as i32;

        let enemy_location = game_state_filters::coordinates_any(ctx, enemy_entity_id);

        let mut contribution_prizes: Vec<(i32, i32, bool)> = ctx
            .db
            .contribution_loot_desc()
            .enemy_type_id()
            .filter(enemy_type_id)
            .map(|c| (c.minimum_contribution, c.item_list_id, c.weighted))
            .collect();
        contribution_prizes.sort_by_key(|c1| c1.0);

        if contribution_prizes.is_empty() {
            log::error!("No contribution available for enemy {enemy_entity_id} (enemy_type_id = {enemy_type_id})");
            return;
        }

        let single_roll = !contribution_prizes.first().unwrap().2; // unweighted contribution = single roll

        let item_lists: HashMap<i32, ItemListDesc> = contribution_prizes
            .iter()
            .map(|c| (c.1, ctx.db.item_list_desc().id().find(c.1).unwrap()))
            .collect();

        let mut is_auto_collect_item_id: HashMap<i32, bool> = HashMap::new();

        for contribution_state in ctx.db.contribution_state().enemy_entity_id().filter(enemy_entity_id) {
            if ctx
                .db
                .player_state()
                .entity_id()
                .find(contribution_state.player_entity_id)
                .is_none()
            {
                //Player is no longer on this region
                continue;
            }

            let mut i = 0;
            let contribution = contribution_state.contribution.ceil() as i32;
            while i + 1 < contribution_prizes.len() && contribution >= contribution_prizes[i + 1].0 {
                i += 1;
            }
            let item_list_id = contribution_prizes[i].1;
            let empty_vec = Vec::new();
            let mut rewards = item_lists[&item_list_id].roll(ctx, if single_roll { 1 } else { contribution }, empty_vec, Some(contribution_state.player_entity_id));
            let mut auto_collected_by_player: HashSet<i32> = HashSet::new();

            // Collect auto-collect outputs
            if !rewards.is_empty() {
                for j in (0..=rewards.len() - 1).rev() {
                    let mut reward = rewards[j];
                    if reward.item_type == ItemType::Item {
                        // cache any item id so we don't make excessive wasm calls
                        is_auto_collect_item_id.entry(reward.item_id).or_insert_with(|| reward.is_auto_collect(ctx));
                        if is_auto_collect_item_id[&reward.item_id] {
                            // cache any collected item id (per player) so we don't make excessive wasm calls
                            if !auto_collected_by_player.contains(&reward.item_id) {
                                let mut discovery = Discovery::new(contribution_state.player_entity_id);
                                if reward.auto_collect(ctx, &mut discovery, contribution_state.player_entity_id) {
                                    discovery.commit(ctx);
                                }
                                auto_collected_by_player.insert(reward.item_id);
                            }
                            rewards.remove(j);
                        }
                    }
                }
                if let Err(message) = InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                    ctx,
                    contribution_state.player_entity_id,
                    &rewards,
                    |x| enemy_location.distance_to(x),
                    true,
                    || vec![{ enemy_location }],
                    false,
                ) {
                    log::error!(
                        "Failed to spawn contribution loot for player {} at location {} error: {}",
                        contribution_state.player_entity_id,
                        enemy_location,
                        message
                    )
                }
            }
        }
    }
}
