use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::game_state_filters;
use crate::game::handlers::player::player_death::player_death_timer;
use crate::game::handlers::server::building_despawn::building_despawn_timer;
use crate::game::handlers::server::enemy_despawn::enemy_despawn_timer;
use crate::{enemy_state, health_state, interior_collapse_trigger_state, parameters_desc, player_state, threat_state};
use crate::{
    game::{
        discovery::Discovery,
        entities::buff,
        game_state,
        handlers::{
            player::player_death::PlayerDeathTimer,
            server::{building_despawn::BuildingDespawnTimer, enemy_despawn::EnemyDespawnTimer},
        },
        terrain_chunk::TerrainChunkCache,
    },
    BuffCategory, BuffDesc, ThreatState,
};

use super::interior_helpers::interior_trigger_collapse;
use super::timer_helpers::now_plus_secs;
use crate::messages::components::{
    active_buff_state, duel_state, player_action_state, rez_sick_long_term_state, DuelState, HealthState, PlayerActionType,
    RezSickLongTermState,
};

pub fn update_health_and_check_death(
    ctx: &ReducerContext,
    _terrain_cache: &mut TerrainChunkCache,
    mut health_state: HealthState,
    entity_id: u64,
    attacker_entity_id: Option<u64>,
) {
    let m = game_state::unix(ctx.timestamp);

    // Ignore this function if still alive or if this new death occurs before respawning
    if !health_state.is_incapacitated_self()
        || m < health_state.died_timestamp + ctx.db.parameters_desc().version().find(0).unwrap().respawn_seconds
    {
        // We still need to do the update for any preceding health update
        ctx.db.health_state().entity_id().update(health_state);
        return;
    }

    // Don't allow a player to die while climbing or using an elevator
    if ctx.db.player_action_state().entity_id().filter(entity_id).any(|action_state| {
        action_state.action_type == PlayerActionType::Climb || action_state.action_type == PlayerActionType::UseElevator
    }) {
        return;
    }

    // Stamp entity death moment
    health_state.died_timestamp = m;
    ctx.db.health_state().entity_id().update(health_state);

    // Nobody can target this entity anymore
    game_state_filters::untarget(ctx, entity_id);

    // Player death specifics. We will leave unsigned players as well to be safe and avoid some zombie case
    let mut long_term_debuffs: Vec<BuffDesc> = BuffDesc::filter_by_buff_category(ctx, BuffCategory::RezSicknessLongTerm).collect();
    long_term_debuffs.sort_by_key(|a| a.priority);

    if ctx.db.player_state().entity_id().find(entity_id).is_some() {
        let mut apply_debuffs = true;

        if let Some(mut duel) = DuelState::get_for_player(ctx, entity_id) {
            if attacker_entity_id.is_some() && duel.player_entity_ids.contains(&attacker_entity_id.unwrap()) {
                // killed by the duel opponent
                apply_debuffs = false;
            }
            // Set loser right away in case the other player gets killed before the second ends and ends up losing because of random order
            duel.set_loser(ctx, duel.player_entity_ids.iter().position(|p| *p == entity_id).unwrap());
            ctx.db.duel_state().entity_id().update(duel);
        }

        // add rez sickness debuffs
        if apply_debuffs {
            let active_buff_state = ctx.db.active_buff_state().entity_id().find(entity_id).unwrap();
            if let Some(debuff) = active_buff_state.active_buff_of_type(ctx, BuffCategory::RezSicknessLongTerm as i32) {
                // Upgrade buff category
                let index = (long_term_debuffs.iter().position(|rs| rs.id == debuff.buff_id).unwrap() + 1).min(long_term_debuffs.len() - 1);
                // Replace the long term buff by its superior version
                let _ = buff::activate(ctx, entity_id, long_term_debuffs[index].id, None, None);
            } else {
                // Provide weakest long term buff
                let _ = buff::activate(ctx, entity_id, long_term_debuffs[0].id, None, None);
            }

            if ctx.db.rez_sick_long_term_state().entity_id().find(entity_id).is_none() {
                ctx.db.rez_sick_long_term_state().insert(RezSickLongTermState { entity_id });
            }

            // Refresh short term debuff
            let rez_sickness_short_term_buff_id = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::RezSicknessShortTerm)
                .unwrap()
                .id;
            let _ = buff::activate(ctx, entity_id, rez_sickness_short_term_buff_id, None, None);
        }

        // Clear all combat sessions involving the dead player
        ThreatState::clear_all(ctx, entity_id);

        // Schedule player death as a separate transaction. This is mostly for client side
        // handling of the player death event. Ideally we shouldn't do this.
        ctx.db
            .player_death_timer()
            .try_insert(PlayerDeathTimer {
                scheduled_at: ctx.timestamp.into(),
                scheduled_id: 0,
                player_entity_id: entity_id,
            })
            .ok()
            .unwrap();

        return;
    }

    // Enemy death specifics - enemies can only die if there is an attacker (slight optimization to avoid an enemy state filter)
    if let Some(attacker_entity_id) = attacker_entity_id {
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(entity_id) {
            let enemy_type = enemy.enemy_type as i32;
            // Grant discovery to everyone engaged in the fight
            for player_entity_id in ctx.db.threat_state().target_entity_id().filter(entity_id).filter_map(|t| {
                match ctx.db.player_state().entity_id().find(t.owner_entity_id) {
                    Some(p) => Some(p.entity_id),
                    None => None,
                }
            }) {
                let mut discovery = Discovery::new(player_entity_id);
                discovery.acquire_enemy(ctx, enemy_type);
                discovery.commit(ctx);
            }

            // Collapse interiors if herd reaches 0 population
            if let Some(collapse_trigger) = ctx.db.interior_collapse_trigger_state().entity_id().find(enemy.herd_entity_id) {
                // Check if all remaining herd entities have 0 hp - at the very least, this one has 0 hp since it hasn't been despawned
                // (it's possible that an AOE attack kills multiple enemies before they have time to despawn, so we can't check only for this one)
                if ctx
                    .db
                    .enemy_state()
                    .herd_entity_id()
                    .filter(enemy.herd_entity_id)
                    .all(|e| ctx.db.health_state().entity_id().find(e.entity_id).unwrap().health <= 0.0)
                {
                    interior_trigger_collapse(ctx, collapse_trigger.dimension_network_entity_id).unwrap();
                }
            }

            // Clear all threat states from anyone targeting the dead entity (feels more responsive for UI)
            // We need to keep the entity's threat state for loot spawning owner later.
            ThreatState::clear_others(ctx, entity_id);

            // Leave some time for the enemy to play its death animation,
            ctx.db
                .enemy_despawn_timer()
                .try_insert(EnemyDespawnTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs(3, ctx.timestamp),
                    entity_id,
                    attacker_entity_id,
                })
                .ok()
                .unwrap();
        } else {
            // Despawn building IN A DIFFERENT TRANSACTION so the AttackOutput stays available.
            ctx.db
                .building_despawn_timer()
                .try_insert(BuildingDespawnTimer {
                    scheduled_at: ctx.timestamp.into(),
                    scheduled_id: 0,
                    entity_id,
                })
                .ok()
                .unwrap();
        }
    }

    // ToDo: buildings, possibly
}
