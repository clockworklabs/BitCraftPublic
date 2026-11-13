use std::time::Duration;

use spacetimedb::{ReducerContext, Table, Timestamp};

pub use crate::game::coordinates::*;
use crate::game::game_state;
use crate::messages::components::*;
use crate::messages::static_data::EnemyType;
use crate::{parameters_desc, params, EnemyDesc};

impl EnemyState {
    pub fn new(ctx: &ReducerContext, enemy_type: EnemyType, herd_entity_id: u64) -> EnemyState {
        let entity_id = game_state::create_entity(ctx);

        EnemyState {
            entity_id,
            enemy_type,
            herd_entity_id,
            direction: HexDirection::random(ctx) as i32,
            last_ranged_attack_timestamp: Timestamp::UNIX_EPOCH,
            status: EnemyStatus::Inactive,
        }
    }

    pub fn enemy_type(&self) -> EnemyType {
        self.enemy_type
    }

    pub fn spawn_enemy(
        ctx: &ReducerContext,
        enemy_desc: &EnemyDesc,
        enemy: EnemyState,
        offset: OffsetCoordinatesSmall,
        herd: Option<&HerdState>,
    ) -> Result<(), String> {
        let entity_id = enemy.entity_id;

        // +Location
        game_state::insert_location_float(ctx, enemy.entity_id, FloatHexTile::from(SmallHexTile::from(offset)).into());

        // +Health
        let health_state = HealthState {
            entity_id,
            health: enemy_desc.max_health as f32,
            last_health_decrease_timestamp: ctx.timestamp,
            died_timestamp: 0,
        };
        ctx.db.health_state().try_insert(health_state)?;

        if let Some(herd) = herd {
            // +EnemyHerdInfoState
            let herd_info = EnemyMobMonitorState {
                entity_id,
                enemy_type: enemy.enemy_type,
                herd_location: ctx
                    .db
                    .location_state()
                    .entity_id()
                    .find(herd.entity_id)
                    .unwrap()
                    .offset_coordinates(),
                herd_entity_id: herd.entity_id,
            };
            ctx.db.enemy_mob_monitor_state().try_insert(herd_info)?;

            // Add contribution lock for prize enemies
            if herd.crumb_trail_entity_id != 0 {
                let lock = CrumbTrailContributionLockState {
                    entity_id,
                    crumb_trail_entity_id: herd.crumb_trail_entity_id,
                };
                ctx.db.crumb_trail_contribution_lock_state().insert(lock);

                let combat_immunity = CombatImmunityState {
                    entity_id,
                    immunity_end_timestamp: ctx.timestamp + Duration::from_secs(params!(ctx).prospecting_herd_immunity_secs as u64),
                    crumb_trail_entity_id: Some(herd.crumb_trail_entity_id),
                };
                ctx.db.combat_immunity_state().insert(combat_immunity);
            }
        }

        // +Targetable
        ctx.db.targetable_state().try_insert(TargetableState::new(entity_id))?;

        // +Combat
        let attack_ids = vec![1, 100]; // For now, all enemies have the "CHARGE" and "AGGRO" attacks

        // Add a combat state for the enemy. Even if not in combat, we need to know the last timestamp of its last action so we can't delete it.
        let combat_state = CombatState::new(entity_id, attack_ids);
        ctx.db.combat_state().try_insert(combat_state)?;

        // Ability States (previous action states) are now created on-the-fly on first use of them by the mob monitor

        ctx.db.attack_outcome_state().try_insert(AttackOutcomeState::new(entity_id))?;

        // +ActiveBuffs (none by default)
        let active_buff_state = ActiveBuffState {
            entity_id,
            active_buffs: Vec::new(),
        };
        ctx.db.active_buff_state().insert(active_buff_state);

        ctx.db.enemy_state().try_insert(enemy)?;

        Ok(())
    }

    pub fn refresh_ranged_attack_timestamp(ctx: &ReducerContext, actor_id: u64, timestamp: Timestamp) {
        if let Some(mut entry) = ctx.db.enemy_state().entity_id().find(actor_id) {
            entry.last_ranged_attack_timestamp = timestamp;
            ctx.db.enemy_state().entity_id().update(entry);
        }
    }
}
