use std::time::Duration;

use spacetimedb::log::{self};
use spacetimedb::rand::Rng;
use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::FloatHexTile;
use crate::game::entities::buff;
use crate::game::game_state::game_state_filters::chunk_indexes_in_radius;
use crate::game::game_state::{self, game_state_filters};
use crate::game::handlers::authentication::has_role;
use crate::game::reducer_helpers::timer_helpers::now_plus_secs_f32;
use crate::game::reducer_helpers::{health_helpers, player_action_helpers};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::action_request::EntityAttackRequest;
use crate::messages::authentication::{Role, ServerIdentity};
use crate::messages::components::*;
use crate::messages::static_data::*;
use crate::unwrap_or_err;

const PLAYER_RADIUS: f32 = 0.5;

#[spacetimedb::reducer]
pub fn attack_start(ctx: &ReducerContext, request: EntityAttackRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        if request.attacker_entity_id != game_state::actor_id(&ctx, false)? {
            return Err("Unauthorized".into());
        }
        game_state::ensure_signed_in(ctx, request.attacker_entity_id)?;
    }

    let delay = event_delay(ctx, request.combat_action_id);

    if request.attacker_type == EntityType::Enemy {
        if ctx.db.enemy_state().entity_id().find(&request.attacker_entity_id).is_none() {
            // Enemy must have been despawned before this attack got processed
            return Err("Attacker is enemy but is missing EnemyState".into());
        }

        // Enemies need to schedule the attack outcome
        if base_checks(
            ctx,
            request.attacker_entity_id,
            request.defender_entity_id,
            request.combat_action_id,
            request.attacker_type,
            request.defender_type,
            true,
        )
        .is_ok()
        {
            // We need to update enemy CombatState to animate their attacks on the client.
            let mut combat = unwrap_or_err!(
                ctx.db.combat_state().entity_id().find(&request.attacker_entity_id),
                "Unable to find attacker's combat state"
            );
            combat.last_attacked_timestamp = game_state::unix_ms(ctx.timestamp);
            ctx.db.combat_state().entity_id().update(combat);

            ctx.db
                .attack_timer()
                .try_insert(AttackTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(delay, ctx.timestamp),
                    attacker_entity_id: request.attacker_entity_id,
                    defender_entity_id: request.defender_entity_id,
                    combat_action_id: request.combat_action_id,
                    attacker_type: request.attacker_type,
                    defender_type: request.defender_type,
                })
                .ok()
                .unwrap();
        }
    } else if request.attacker_type == EntityType::Player {
        // Players will send another attack notification after the wind-up
        let attacker_id = request.attacker_entity_id;
        HealthState::check_incapacitated(ctx, attacker_id, false)?;

        if let Some(ms) = ctx.db.mounting_state().entity_id().find(attacker_id) {
            let deployable_state = unwrap_or_err!(
                ctx.db.deployable_state().entity_id().find(ms.deployable_entity_id),
                "Deployable doesn't exist"
            );
            let deployable_desc = unwrap_or_err!(
                ctx.db.deployable_desc_v4().id().find(deployable_state.deployable_description_id),
                "DeployableDesc doesn't exist"
            );
            let enemy_state = unwrap_or_err!(
                ctx.db.enemy_state().entity_id().find(request.defender_entity_id),
                "Cannot attack while mounted"
            );
            let enemy_desc = unwrap_or_err!(
                ctx.db.enemy_desc().enemy_type().find(enemy_state.enemy_type as i32),
                "Invalid enemy type"
            );
            if !enemy_desc.huntable || !deployable_desc.allow_hunting {
                return Err("Cannot attack while mounted".into());
            }
        }

        PlayerTimestampState::refresh(ctx, attacker_id, ctx.timestamp);
        let target = Some(request.defender_entity_id);

        // only do the checks for the main target
        return player_action_helpers::start_action(
            ctx,
            attacker_id,
            PlayerActionType::Attack,
            target,
            Some(request.combat_action_id),
            Duration::from_secs_f32(delay),
            base_checks(
                ctx,
                request.attacker_entity_id,
                request.defender_entity_id,
                request.combat_action_id,
                request.attacker_type,
                request.defender_type,
                true,
            ),
            game_state::unix_ms(ctx.timestamp),
        );
    } else {
        return Err("Neither a Player nor Enemy is attacking.".into());
    }
    Ok(())
}

#[spacetimedb::table(name = attack_timer, public, scheduled(attack_scheduled, at = scheduled_at))]
pub struct AttackTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub attacker_entity_id: u64,
    pub defender_entity_id: u64,
    pub combat_action_id: i32,
    pub attacker_type: EntityType,
    pub defender_type: EntityType,
}

#[spacetimedb::reducer]
fn attack_scheduled(ctx: &ReducerContext, timer: AttackTimer) -> Result<(), String> {
    attack(
        ctx,
        EntityAttackRequest {
            attacker_entity_id: timer.attacker_entity_id,
            defender_entity_id: timer.defender_entity_id,
            combat_action_id: timer.combat_action_id,
            attacker_type: timer.attacker_type,
            defender_type: timer.defender_type,
        },
    )
}

fn targetable_entities_in_radius(
    ctx: &ReducerContext,
    attacker_entity_id: u64,
    attacker_type: EntityType,
    coord: FloatHexTile,
    radius: f32,
) -> Vec<(u64, EntityType, f32)> {
    let attacker_radius;
    let attacker_level;
    if attacker_type == EntityType::Enemy {
        let enemy_type = ctx.db.enemy_state().entity_id().find(attacker_entity_id).unwrap().enemy_type;
        let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type as i32).unwrap();
        attacker_level = enemy_desc.attack_level;
        attacker_radius = enemy_desc.radius;
    } else {
        attacker_level = i32::MAX;
        attacker_radius = PLAYER_RADIUS;
    };
    let scan_radius = radius + attacker_radius + 8.0; // since we don't know the type of entities we will find, and therefore their radius, we will scan with an extra 8 units (should be larger than the largest radius)

    // Gather players and enemies around attacker
    let attacker_targeting_matrix = if attacker_type == EntityType::Enemy {
        TargetingMatrixDesc::from_enemy_entity_id(ctx, attacker_entity_id).ok().unwrap()
    } else {
        TargetingMatrixDesc::player(ctx)
    };

    let players_and_enemies = chunk_indexes_in_radius(coord.into(), radius.ceil() as i32)
        .flat_map(|chunk_index| ctx.db.mobile_entity_state().chunk_index().filter(chunk_index))
        .filter_map(|mes| {
            if mes.entity_id != attacker_entity_id {
                let dist = mes.coordinates_float().distance_to(coord);
                if dist <= scan_radius {
                    match ctx.db.enemy_state().entity_id().find(&mes.entity_id) {
                        Some(enemy) => {
                            let defender_targeting_matrix = TargetingMatrixDesc::from_enemy_entity_id(ctx, enemy.entity_id).ok().unwrap();
                            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy.enemy_type as i32).unwrap();
                            if !attacker_targeting_matrix.can_attack(&defender_targeting_matrix)
                                || enemy_desc.defense_level > enemy_desc.attack_level
                                || dist > radius + attacker_radius + enemy_desc.radius
                                || ctx
                                    .db
                                    .health_state()
                                    .entity_id()
                                    .find(mes.entity_id)
                                    .unwrap()
                                    .is_incapacitated_self()
                            {
                                None // Not an attackable type, or defense level is too high or too far or it's already dead
                            } else {
                                Some((enemy.entity_id, EntityType::Enemy, dist))
                            }
                        }
                        None => {
                            if ctx.db.signed_in_player_state().entity_id().find(&mes.entity_id).is_some() {
                                let defender_targeting_matrix = TargetingMatrixDesc::player(ctx);
                                if !attacker_targeting_matrix.can_attack(&defender_targeting_matrix)
                                    || dist > radius + attacker_radius + PLAYER_RADIUS
                                    || CharacterStatsState::get_entity_stat(ctx, mes.entity_id, CharacterStatType::DefenseLevel) as i32
                                        > attacker_level
                                    || ctx
                                        .db
                                        .health_state()
                                        .entity_id()
                                        .find(mes.entity_id)
                                        .unwrap()
                                        .is_incapacitated_self()
                                {
                                    None // Not an attackable type, or defense level is too high or too far or it's already dead
                                } else {
                                    if let Some(active_buff_state) = ctx.db.active_buff_state().entity_id().find(mes.entity_id) {
                                        if !active_buff_state.has_innerlight_buff(ctx) {
                                            Some((mes.entity_id, EntityType::Player, dist))
                                        } else {
                                            None // Player is protected by inner light
                                        }
                                    } else {
                                        // No buff state for player, therefore no inner light
                                        Some((mes.entity_id, EntityType::Player, dist))
                                    }
                                }
                            } else {
                                None // No player and no enemy, not targetable
                            }
                        }
                    }
                } else {
                    None // too far
                }
            } else {
                None // self-target
            }
        })
        .collect();

    // Note: currently enemies and players don't attack buildings so we are not handling it in the multi-targeting.
    // This would require a location check in several chunks, so we'd probably need to add the chunk_index as part of the building state for indexation
    players_and_enemies
}

#[spacetimedb::reducer]
pub fn attack(ctx: &ReducerContext, request: EntityAttackRequest) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        if request.attacker_entity_id != game_state::actor_id(&ctx, false)? {
            return Err("Unauthorized".into());
        }
        game_state::ensure_signed_in(ctx, request.attacker_entity_id)?;
        HealthState::check_incapacitated(ctx, request.attacker_entity_id, false)?;
    }

    let combat_action = unwrap_or_err!(
        ctx.db.combat_action_desc_v3().id().find(&request.combat_action_id),
        "Combat action doesn't exist"
    );

    let mut targets = vec![request.defender_entity_id];
    let mut target_types = vec![request.defender_type];

    if let Some(action) = ctx.db.combat_action_multi_hit_desc().id().find(request.combat_action_id) {
        // only multi-targeting actions need to calculate targets at the end
        let attacker_coord = game_state_filters::coordinates_any_float(ctx, request.attacker_entity_id);
        let mut entities = targetable_entities_in_radius(
            ctx,
            request.attacker_entity_id,
            request.attacker_type,
            attacker_coord,
            combat_action.max_range,
        );
        entities.sort_by_key(|a| (a.2 * 10000.0) as i32);
        for (entity_id, entity_type, _) in entities
            .iter()
            .filter(|x| x.0 != request.defender_entity_id)
            .take(action.max_secondary_targets as usize)
        {
            targets.push(*entity_id);
            target_types.push(*entity_type);
        }
    }

    for i in 0..targets.len() {
        let defender_entity_id = targets[i];
        let defender_type = target_types[i];

        let launch_result = attack_reduce(
            ctx,
            request.attacker_entity_id,
            defender_entity_id,
            request.combat_action_id,
            request.attacker_type,
            defender_type,
            i == 0,
        );
        if request.attacker_type == EntityType::Player {
            HealthState::check_incapacitated(ctx, request.attacker_entity_id, false)?;
            PlayerTimestampState::refresh(ctx, request.attacker_entity_id, ctx.timestamp);
            player_action_helpers::schedule_clear_player_action(
                request.attacker_entity_id,
                PlayerActionType::Attack.get_layer(ctx),
                launch_result.clone(),
            )?;
        }
        if request.attacker_type == EntityType::Enemy {
            EnemyState::refresh_ranged_attack_timestamp(ctx, request.attacker_entity_id, ctx.timestamp);
        }

        if launch_result.is_err() {
            if targets.len() == 1 && request.attacker_type == EntityType::Player {
                // Additionally check for player, bit of a hack, but we want ground slam from enemies to execute with errors if everyones out of range.
                return launch_result;
            }
            // Execute attack on all potential targets
            continue;
        }

        let is_instant_attack = combat_action.projectile_speed <= 0.0;
        if is_instant_attack {
            // Don't want to call attack_impact_reduce directly because we can't respond to a non-reducer on the client.
            ctx.db
                .attack_impact_timer_migrated()
                .try_insert(AttackImpactTimerMigrated {
                    scheduled_id: 0,
                    scheduled_at: ctx.timestamp.into(),
                    attacker_entity_id: request.attacker_entity_id,
                    defender_entity_id,
                    combat_action_id: request.combat_action_id,
                    attacker_type: request.attacker_type,
                    defender_type,
                    main_attack: i == 0,
                })
                .map_err(|_e| String::from("Unique constraint violation"))?;
        } else {
            let distance = game_state_filters::coordinates_any(ctx, request.attacker_entity_id)
                .distance_to(game_state_filters::coordinates_any(ctx, request.defender_entity_id));
            let projectile_time = (distance as f32) / combat_action.projectile_speed;
            ctx.db
                .attack_impact_timer_migrated()
                .try_insert(AttackImpactTimerMigrated {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(projectile_time, ctx.timestamp),
                    attacker_entity_id: request.attacker_entity_id,
                    defender_entity_id,
                    combat_action_id: request.combat_action_id,
                    attacker_type: request.attacker_type,
                    defender_type,
                    main_attack: i == 0,
                })
                .map_err(|_e| String::from("Unique constraint violation"))?;
        }
    }
    Ok(())
}

// OBSOLETE
#[spacetimedb::table(name = attack_impact_timer, scheduled(attack_impact, at = scheduled_at))]
pub struct AttackImpactTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub attacker_entity_id: u64,
    pub defender_entity_id: u64,
    pub combat_action_id: i32,
    pub attacker_type: EntityType,
    pub defender_type: EntityType,
}

// [MIGRATION WORK-AROUND] This should be AttackImpactTimer with a main_attack field added
#[spacetimedb::table(name = attack_impact_timer_migrated, public, scheduled(attack_impact_migrated, at = scheduled_at), index(name = attacker_entity_id, btree(columns = [attacker_entity_id])))]
pub struct AttackImpactTimerMigrated {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub attacker_entity_id: u64,
    pub defender_entity_id: u64,
    pub combat_action_id: i32,
    pub attacker_type: EntityType,
    pub defender_type: EntityType,
    pub main_attack: bool,
}

#[spacetimedb::reducer]
pub fn attack_impact(ctx: &ReducerContext, _timer: AttackImpactTimer) -> Result<(), String> {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        return Err("Invalid identity".into());
    }

    panic!("DEPRECATED.");
}

#[spacetimedb::reducer]
pub fn attack_impact_migrated(ctx: &ReducerContext, timer: AttackImpactTimerMigrated) -> Result<(), String> {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        return Err("Invalid identity".into());
    }

    attack_impact_reduce(
        ctx,
        timer.attacker_entity_id,
        timer.defender_entity_id,
        timer.combat_action_id,
        timer.attacker_type,
        timer.defender_type,
        timer.main_attack,
    )
}

fn event_delay(ctx: &ReducerContext, combat_action_id: i32) -> f32 {
    match ctx.db.combat_action_desc_v3().id().find(&combat_action_id) {
        Some(action) => action.lead_in_time,
        None => 0.0,
    }
}

fn attack_reduce(
    ctx: &ReducerContext,
    attacker_entity_id: u64,
    defender_entity_id: u64,
    combat_action_id: i32,
    attacker_type: EntityType,
    defender_type: EntityType,
    main_attack: bool,
) -> Result<(), String> {
    base_checks(
        ctx,
        attacker_entity_id,
        defender_entity_id,
        combat_action_id,
        attacker_type,
        defender_type,
        main_attack,
    )?;

    let attacker_health = unwrap_or_err!(
        ctx.db.health_state().entity_id().find(&attacker_entity_id),
        "Attacker no longer exists"
    );
    if attacker_health.health == 0.0 {
        return Err("Attacker is dead.".into());
    }
    let combat_action = unwrap_or_err!(
        ctx.db.combat_action_desc_v3().id().find(&combat_action_id),
        "Combat action doesn't exist"
    );

    let now = game_state::unix_ms(ctx.timestamp);

    // At this point the ability state necessarily exists, else base_checks would have failed
    let mut ability_state = unwrap_or_err!(
        ctx.db
            .ability_state()
            .owner_entity_id()
            .filter(attacker_entity_id)
            .filter(|a| a.ability == AbilityType::CombatAction(combat_action_id))
            .next(),
        "This ability is not known by the attacker"
    );

    let ability_entity_id = ability_state.entity_id;

    let mut cooldown_multiplier = 1.0;
    let mut weapon_cooldown_multiplier = 1.0;

    if attacker_type == EntityType::Enemy {
        if ctx.db.enemy_state().entity_id().find(&attacker_entity_id).is_some() {
            let collected_stats = ActiveBuffState::collect_enemy_stats(ctx, attacker_entity_id);
            cooldown_multiplier = ActiveBuffState::get_enemy_stat(&collected_stats, CharacterStatType::CooldownMultiplier, 1);
        }
    } else {
        // Default is combat weapon, unless it's a huntable enemy
        let mut is_huntable = false;
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(defender_entity_id) {
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy.enemy_type as i32).unwrap();
            is_huntable = enemy_desc.huntable;
        }
        (cooldown_multiplier, weapon_cooldown_multiplier) =
            CharacterStatsState::get_cooldown_and_weapon_cooldown_multipliers(ctx, attacker_entity_id, is_huntable);
    }
    ability_state.set_combat_action_cooldown(
        &combat_action,
        cooldown_multiplier,
        weapon_cooldown_multiplier,
        ctx.timestamp,
        false,
    );
    ctx.db.ability_state().entity_id().update(ability_state);

    let is_attacking_player = attacker_type == EntityType::Player;
    if is_attacking_player {
        if main_attack {
            // Make sure target and timestamp and action fit
            PlayerActionState::validate(ctx, attacker_entity_id, PlayerActionType::Attack, Some(defender_entity_id))?;
        }

        // Todo: eventually we can cache the stamina component and save 1 extra filter
        StaminaState::decrease_stamina(ctx, attacker_entity_id, -combat_action.stamina_use);

        // Update attacker's combat state when the attack hits
        let mut combat = unwrap_or_err!(
            ctx.db.combat_state().entity_id().find(&attacker_entity_id),
            "Unable to find attacker's combat state"
        );

        combat.last_attacked_timestamp = game_state::unix_ms(ctx.timestamp);
        combat.last_performed_action_entity_id = ability_entity_id;

        combat.global_cooldown = if combat_action.global_cooldown / cooldown_multiplier == 0.0 {
            None
        } else {
            Some(ActionCooldown {
                timestamp: now,
                cooldown: combat_action.global_cooldown / cooldown_multiplier,
            })
        };

        ctx.db.combat_state().entity_id().update(combat);

        // remove innerlight from attacker
        if ctx.db.active_buff_state().entity_id().find(&attacker_entity_id).is_some() {
            let inner_light = BuffDesc::find_by_buff_category_single(ctx, BuffCategory::InnerLight).unwrap().id;
            buff::deactivate(ctx, attacker_entity_id, inner_light).unwrap();
        }
    }

    // produce attack buffs and debuffs
    if main_attack && combat_action.self_buffs.len() > 0 {
        let mut active_buff_state = unwrap_or_err!(
            ctx.db.active_buff_state().entity_id().find(attacker_entity_id),
            "Entity has no ActiveBuffState."
        );
        for buff_effect in combat_action.self_buffs {
            active_buff_state.add_active_buff_with_data(ctx, buff_effect.buff_id, buff_effect.duration, None);
        }
        ctx.db.active_buff_state().entity_id().update(active_buff_state);
    }

    Ok(())
}

fn attack_impact_reduce(
    ctx: &ReducerContext,
    attacker_entity_id: u64,
    defender_entity_id: u64,
    combat_action_id: i32,
    attacker_type: EntityType,
    defender_type: EntityType,
    main_attack: bool,
) -> Result<(), String> {
    let combat_action = unwrap_or_err!(
        ctx.db.combat_action_desc_v3().id().find(&combat_action_id),
        "Combat action doesn't exist"
    );
    let mut defender_health = unwrap_or_err!(
        ctx.db.health_state().entity_id().find(&defender_entity_id),
        "Unable to find attacked entity health"
    );
    if defender_health.health == 0.0 {
        // Enemy already dead - nothing to do
        return Ok(());
    }
    // roll outcome
    let (damage, scaled_damage, dodged, critical) = calculate_hit_outcome(
        ctx,
        attacker_entity_id,
        attacker_type,
        defender_entity_id,
        defender_type,
        combat_action_id,
        main_attack,
    );
    let mut attack_outcome = ctx.db.attack_outcome_state().entity_id().find(&defender_entity_id).unwrap();
    attack_outcome.set(damage, critical, dodged, ctx.timestamp);
    ctx.db.attack_outcome_state().entity_id().update(attack_outcome);
    if !dodged {
        if combat_action.target_buffs.len() > 0 {
            let mut active_buff_state = unwrap_or_err!(
                ctx.db.active_buff_state().entity_id().find(defender_entity_id),
                "Entity has no ActiveBuffState."
            );
            for buff_effect in combat_action.target_buffs {
                active_buff_state.add_active_buff_with_data(ctx, buff_effect.buff_id, buff_effect.duration, None);
            }
            ctx.db.active_buff_state().entity_id().update(active_buff_state);
        }
        if damage > 0 {
            // Damage calculation and health reduction
            if defender_health.health > 0.0 {
                let ignore_damage = match defender_type {
                    EntityType::Enemy => {
                        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(defender_entity_id) {
                            ctx.db
                                .enemy_desc()
                                .enemy_type()
                                .find(enemy.enemy_type as i32)
                                .unwrap()
                                .ignore_damage
                        } else {
                            false
                        }
                    }
                    EntityType::Building => {
                        if let Some(building) = ctx.db.building_state().entity_id().find(defender_entity_id) {
                            ctx.db
                                .building_desc()
                                .id()
                                .find(building.building_description_id)
                                .unwrap()
                                .ignore_damage
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if !ignore_damage {
                    if attacker_type == EntityType::Player && defender_type == EntityType::Enemy {
                        if ContributionState::applies(ctx, defender_entity_id) {
                            ContributionState::add_damage(
                                ctx,
                                attacker_entity_id,
                                defender_entity_id,
                                (damage as f32).min(defender_health.health).ceil() as i32,
                            );
                        }
                    }
                    defender_health.add_health_delta(-scaled_damage as f32, ctx.timestamp);
                }
                // Apply any death event for slain entity.
                let mut terrain_cache = TerrainChunkCache::empty();

                // Only award experience against the main target
                if main_attack {
                    game_state_filters::award_experience_on_damage(ctx, damage as f32, defender_entity_id, Some(attacker_entity_id));
                }

                if !ignore_damage {
                    health_helpers::update_health_and_check_death(
                        ctx,
                        &mut terrain_cache,
                        defender_health,
                        defender_entity_id,
                        Some(attacker_entity_id),
                    );
                }
            }
        }
    }
    let self_threat = if defender_type == EntityType::Building {
        combat_action.self_threat_against_buildings
    } else {
        combat_action.self_threat_against_enemies
    };
    ThreatState::add_threat(ctx, attacker_entity_id, defender_entity_id, self_threat);
    let threat = combat_action.base_threat + combat_action.threat_per_damage * damage as f32;
    if combat_action.is_taunt_action {
        ThreatState::equalize_threat_then_add(ctx, defender_entity_id, attacker_entity_id, threat);
    } else {
        ThreatState::add_threat(ctx, defender_entity_id, attacker_entity_id, threat);
    }

    // Only drop weapon durability now. Destroying the weapon earlier might destroy the combat actions, resulting in wrong outcome calculation
    if main_attack && combat_action.weapon_durability_lost > 0 {
        let mut is_hunting_weapon = false;
        if defender_type == EntityType::Enemy {
            if let Some(enemy) = ctx.db.enemy_state().entity_id().find(defender_entity_id) {
                let enemy_type = enemy.enemy_type as i32;
                let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();
                is_hunting_weapon = enemy_desc.huntable;
            }
        }

        let tool_type = if is_hunting_weapon {
            PlayerState::get_hunting_weapon_type()
        } else {
            PlayerState::get_combat_weapon_type(ctx)
        };

        InventoryState::reduce_tool_durability(ctx, attacker_entity_id, tool_type, combat_action.weapon_durability_lost);
    }
    Ok(())
}

fn interpolated_position(ctx: &ReducerContext, entity_id: u64) -> FloatHexTile {
    if let Some(target_mobile_entity) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
        target_mobile_entity.interpolated_location(ctx)
    } else {
        game_state_filters::coordinates_any_float(ctx, entity_id)
    }
}

fn range_check(
    ctx: &ReducerContext,
    combat_action: &CombatActionDescV3,
    attacker_location: FloatHexTile,
    attacker_entity_id: u64,
    attacker_type: EntityType,
    defender_entity_id: u64,
    defender_type: EntityType,
) -> bool {
    let defender_radius = get_radius(ctx, defender_type, defender_entity_id);
    let attacker_radius = get_radius(ctx, attacker_type, attacker_entity_id);
    let defender_coord = interpolated_position(ctx, defender_entity_id);
    let distance = attacker_location.distance_to(defender_coord) - defender_radius - attacker_radius;
    distance <= combat_action.max_range
}

fn base_checks(
    ctx: &ReducerContext,
    attacker_entity_id: u64,
    defender_entity_id: u64,
    combat_action_id: i32,
    attacker_type: EntityType,
    defender_type: EntityType,
    main_attack: bool,
) -> Result<(), String> {
    // DAB Note: IF THIS ERRORS, THE CLIENT IS NOT INFORMED.
    // WE WILL NEED TO PASS THE ATTACKER'S IDENTITY AS A SCHEDULED SUBSCRIBER

    let attacker_health = unwrap_or_err!(
        ctx.db.health_state().entity_id().find(&attacker_entity_id),
        "Attacker no longer exists"
    );
    if attacker_health.health == 0.0 {
        return Err("Attacker is dead.".into());
    }

    let combat_action = unwrap_or_err!(
        ctx.db.combat_action_desc_v3().id().find(&combat_action_id),
        "Combat action doesn't exist"
    );

    let attacker_targeting_matrix = match attacker_type {
        EntityType::Player => TargetingMatrixDesc::player(ctx),
        EntityType::Enemy => match TargetingMatrixDesc::from_enemy_entity_id(ctx, attacker_entity_id) {
            Ok(tm) => tm,
            Err(()) => return Err("The attacking entity no longer exists".into()),
        },
        _ => return Err("This entity type cannot attack".into()),
    };

    let defender_targeting_matrix = match defender_type {
        EntityType::Player => TargetingMatrixDesc::player(ctx),
        EntityType::Enemy => match TargetingMatrixDesc::from_enemy_entity_id(ctx, defender_entity_id) {
            Ok(tm) => tm,
            Err(()) => return Err("~The defending entity no longer exists".into()),
        },
        EntityType::Building => TargetingMatrixDesc::building(ctx),
        _ => return Err("This entity type cannot be attacked".into()),
    };

    if attacker_entity_id != defender_entity_id && !attacker_targeting_matrix.can_attack(&defender_targeting_matrix) {
        if !DuelState::are_players_dueling(ctx, attacker_entity_id, defender_entity_id) {
            return Err("Unable to target that entity".into());
        }
    }

    let defender_health = unwrap_or_err!(
        ctx.db.health_state().entity_id().find(&defender_entity_id),
        "Unable to find attacked entity health"
    );

    if defender_health.health == 0.0 {
        // Already dead - silent error
        return Err(String::new());
    }

    let ability_state = match ctx
        .db
        .ability_state()
        .owner_entity_id()
        .filter(attacker_entity_id)
        .filter(|a| a.ability == AbilityType::CombatAction(combat_action_id))
        .next()
    {
        Some(a) => a,
        None => {
            if attacker_type == EntityType::Player {
                // Player abilities are set on the toolbar or added on weapon change (auto-cast).
                // We can't create a new ability_state on the fly for a plyer.
                return Err("You don't know this ability".into());
            }
            // Enemy ability_states are created on the fly when they attack, if not already known.
            let ability_state_to_insert = AbilityState {
                entity_id: game_state::create_entity(ctx),
                owner_entity_id: attacker_entity_id,
                ability: AbilityType::CombatAction(combat_action_id),
                cooldown: ActionCooldown {
                    timestamp: 0,
                    cooldown: 0.0,
                },
            };
            let ability_state_copy = ability_state_to_insert.clone();
            ctx.db.ability_state().insert(ability_state_to_insert);
            ability_state_copy
        }
    };

    // Range check:
    let action_desc = ctx.db.combat_action_desc_v3().id().find(&combat_action.id).unwrap();
    if !range_check(
        ctx,
        &action_desc,
        game_state_filters::coordinates_any_float(ctx, attacker_entity_id),
        attacker_entity_id,
        attacker_type,
        defender_entity_id,
        defender_type,
    ) {
        return Err("Too far".into());
    }

    // Only check timings for players to cull out cheaters. AI shouldn't be cheating.
    if attacker_type == EntityType::Player && main_attack {
        // check if combat action is unlocked
        AbilityUnlockDesc::evaluate(
            ctx,
            attacker_entity_id,
            AbilityTypeEnum::CombatAction,
            AbilityType::CombatAction(action_desc.id),
        )?;

        // Contribution (crumb trail prizes) gating
        if let Some(contribution_lock) = ctx.db.crumb_trail_contribution_lock_state().entity_id().find(defender_entity_id) {
            if let Some(prospecting) = ctx.db.prospecting_state().entity_id().find(attacker_entity_id) {
                if prospecting.crumb_trail_entity_id != contribution_lock.crumb_trail_entity_id {
                    return Err("You must have helped find this quarry to be able to interact with it".into());
                }
            } else {
                if ctx
                    .db
                    .crumb_trail_contribution_spent_state()
                    .player_and_crumb_entity_id()
                    .filter((attacker_entity_id, contribution_lock.crumb_trail_entity_id))
                    .next()
                    .is_some()
                {
                    return Err("You've already interacted all you can with this quarry".into());
                }
                return Err("You must have helped find this quarry to be able to interact with it".into());
            }
        }

        if ability_state.is_under_cooldown(ctx, !combat_action.ignore_global_cooldown) {
            // Under cooldown, silent error
            return Err(String::new());
        }

        let stamina_state = unwrap_or_err!(
            ctx.db.stamina_state().entity_id().find(&attacker_entity_id),
            "Player has no stamina component"
        );

        if stamina_state.stamina < combat_action.stamina_use {
            return Err("Not enough stamina.".into());
        }

        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(&defender_entity_id) {
            let weapon_tier;
            let enemy_type = enemy.enemy_type as i32;
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

            if enemy_desc.huntable {
                if let Some(weapon) = PlayerState::get_hunting_weapon(ctx, attacker_entity_id) {
                    let weapon_desc = ctx.db.weapon_desc().item_id().find(&weapon.item_id).unwrap();
                    if !action_desc.weapon_type_requirements.contains(&weapon_desc.weapon_type) {
                        return Err("You cannot use this action with your currently equipped hunting weapon".into());
                    }
                    weapon_tier = weapon_desc.tier;
                } else {
                    return Err("You need a hunting weapon to attack this type of enemy".into());
                }
            } else {
                if let Some(weapon) = PlayerState::get_combat_weapon(ctx, attacker_entity_id) {
                    let weapon_desc = ctx.db.weapon_desc().item_id().find(&weapon.item_id).unwrap();
                    if !action_desc.weapon_type_requirements.contains(&weapon_desc.weapon_type) {
                        return Err("You cannot use this action with your currently equipped weapon".into());
                    }
                    weapon_tier = weapon_desc.tier;
                } else {
                    return Err("You need a different type of weapon to attack this enemy".into());
                };
            }
            if weapon_tier < enemy_desc.tier {
                return Err(format!("You need a tier {{0}} weapon to attack this type of enemy|~{}", enemy_desc.tier));
            }
        }

        // Validate move + attack, the client should've stopped the player's movement beforehand.
        if !action_desc.can_move_during_lead_in {
            if let Some(mes) = ctx.db.mobile_entity_state().entity_id().find(&attacker_entity_id) {
                if mes.destination_float() != mes.coordinates_float() {
                    // Cannot move and attack - silent error.
                    return Err(String::new());
                }
            }
        }
    }

    Ok(())
}

fn calculate_hit_outcome(
    ctx: &ReducerContext,
    attacker_entity_id: u64,
    attacker_type: EntityType,
    defender_entity_id: u64,
    defender_type: EntityType,
    combat_action_id: i32,
    main_attack: bool,
) -> (i32, i32, bool, bool) {
    const STRENGTH_PER_DAMAGE: f32 = 15.0; // +1 damage per 15 strength
    const EVASION_MULTIPLIER: f32 = 0.1;
    const ARMOR_50PCT_REDUCTION: f32 = 2000.0;
    let verbose = false;

    let strength_stat;
    let armor;
    let mut evasion;
    let mut scaled_armor = 0.0;

    match defender_type {
        EntityType::Enemy => {
            if let Some(e) = ctx.db.enemy_state().entity_id().find(&defender_entity_id) {
                let enemy_type = e.enemy_type as i32;
                let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

                let collected_stats = ActiveBuffState::collect_enemy_stats(ctx, defender_entity_id);
                armor = ActiveBuffState::get_enemy_stat(&collected_stats, CharacterStatType::Armor, enemy_desc.armor).max(0.0);
                evasion = ActiveBuffState::get_enemy_stat(&collected_stats, CharacterStatType::Evasion, enemy_desc.evasion).max(0.0);

                let defender_scaling = match ctx.db.enemy_scaling_state().entity_id().find(defender_entity_id) {
                    Some(scaling_state) => ctx.db.enemy_scaling_desc().id().find(scaling_state.enemy_scaling_id),
                    None => None,
                };
                if let Some(scaling) = defender_scaling {
                    scaled_armor = scaling.scaled_armor_bonus as f32;
                    evasion += scaling.evasion_bonus as f32;
                }
                strength_stat = Some(if enemy_desc.huntable {
                    CharacterStatType::HuntingWeaponPower
                } else {
                    CharacterStatType::Strength
                });
            } else {
                return (0, 0, true, false); // Attack is "evaded" since the defender no longer exists
            }
        }
        EntityType::Building => {
            if ctx.db.building_state().entity_id().find(&defender_entity_id).is_some() {
                armor = 0.0;
                evasion = 0.0;
                strength_stat = Some(CharacterStatType::Strength);
            } else {
                return (0, 0, true, false); // Attack is "evaded" since the defender no longer exists
            }
        }
        _ => {
            if let Some(s) = ctx.db.character_stats_state().entity_id().find(&defender_entity_id) {
                armor = s.get(CharacterStatType::Armor).max(0.0);
                evasion = s.get(CharacterStatType::Evasion).max(0.0);
                strength_stat = None;
            } else {
                return (0, 0, true, false); // Attack is "evaded" since the defender no longer exists
            }
        }
    }

    let mut accuracy;
    let mut strength;
    let weapon_cooldown;
    let damage_roll;

    let strength_stat = strength_stat.unwrap_or(CharacterStatType::Strength);

    if attacker_type == EntityType::Enemy {
        if let Some(e) = ctx.db.enemy_state().entity_id().find(&attacker_entity_id) {
            let enemy_type = e.enemy_type as i32;
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();

            let collected_stats = ActiveBuffState::collect_enemy_stats(ctx, attacker_entity_id);
            accuracy = ActiveBuffState::get_enemy_stat(&collected_stats, CharacterStatType::Accuracy, enemy_desc.accuracy);
            strength = ActiveBuffState::get_enemy_stat(&collected_stats, CharacterStatType::Strength, enemy_desc.strength);

            let attacker_scaling = match ctx.db.enemy_scaling_state().entity_id().find(attacker_entity_id) {
                Some(scaling_state) => ctx.db.enemy_scaling_desc().id().find(scaling_state.enemy_scaling_id),
                None => None,
            };

            if let Some(scaling) = attacker_scaling {
                damage_roll = ctx
                    .rng()
                    .gen_range(enemy_desc.min_damage + scaling.min_damage_bonus..=enemy_desc.max_damage + scaling.max_damage_bonus);
                accuracy += scaling.accuracy_bonus as f32;
                strength += scaling.strength_bonus as f32;
            } else {
                damage_roll = ctx.rng().gen_range(enemy_desc.min_damage..=enemy_desc.max_damage);
            }
            weapon_cooldown = 1.0; // Monsters don't have weapon cooldowns
        } else {
            return (0, 0, true, false); // Attack is "evaded" since the attacker no longer exists
        }
    } else {
        if let Some(s) = ctx.db.character_stats_state().entity_id().find(&attacker_entity_id) {
            accuracy = s.get(CharacterStatType::Accuracy);
            strength = s.get(strength_stat);

            let weapon_id = if strength_stat == CharacterStatType::Strength {
                PlayerState::get_combat_weapon(ctx, attacker_entity_id).unwrap().item_id
            } else {
                PlayerState::get_hunting_weapon(ctx, attacker_entity_id).unwrap().item_id
            };
            let weapon = ctx.db.weapon_desc().item_id().find(weapon_id).unwrap();
            damage_roll = ctx.rng().gen_range(weapon.min_damage..=weapon.max_damage);
            weapon_cooldown = weapon.cooldown;
        } else {
            return (0, 0, true, false); // Attack is "evaded" since the attacker no longer exists
        }
    }

    let combat_action = ctx.db.combat_action_desc_v3().id().find(combat_action_id).unwrap();

    // don't evade self-actions
    if attacker_entity_id == defender_entity_id {
        evasion = 0.0;
    }

    // Evasion can only happen in combat, not in hunting
    if strength_stat == CharacterStatType::Strength {
        // Roll hit outcome
        let attack_roll = ctx.rng().gen_range(0.0..=accuracy * combat_action.accuracy_multiplier);
        let evasion_roll = ctx.rng().gen_range(0.0..=evasion * EVASION_MULTIPLIER);

        if verbose {
            log::info!(
                "Attack Roll => {} (0.0 - {}) (accuracy: {}, multiplier: {})",
                attack_roll,
                accuracy * combat_action.accuracy_multiplier,
                accuracy,
                combat_action.accuracy_multiplier
            );
            log::info!(
                "Evasion Roll => {} (0.0 - {}) (evasion: {}, multiplier: {})",
                evasion_roll,
                evasion * EVASION_MULTIPLIER,
                evasion,
                EVASION_MULTIPLIER
            );
        }

        if attack_roll <= evasion_roll {
            return (0, 0, true, false); // Attack is evaded
        }
    }

    let attack_is_critical = false; // for now there's no critical hit. That's odd.

    // Apply combat action on damage
    let strength = (strength * combat_action.strength_multiplier) / (1.0 / weapon_cooldown);
    let bonus_damage = (strength / STRENGTH_PER_DAMAGE).ceil() as i32;
    let damage = (damage_roll + bonus_damage) as f32;

    // Apply armor on damage
    let damage_reduction = armor / (armor + ARMOR_50PCT_REDUCTION);
    let mut updated_damage = damage * (1.0 - damage_reduction);

    if !main_attack {
        let multi_attack = ctx.db.combat_action_multi_hit_desc().id().find(combat_action_id).unwrap();
        updated_damage *= multi_attack.secondary_target_multiplier;
    }

    let scaled_damage_reduction = scaled_armor / (scaled_armor + ARMOR_50PCT_REDUCTION);
    let scaled_damage_outcome = (updated_damage * (1.0 - scaled_damage_reduction)).floor() as i32;

    let damage_outcome = updated_damage.floor() as i32;

    if verbose {
        log::info!("Damage => {damage} (roll: {damage_roll} + bonus {bonus_damage}");
        log::info!("Damage Reduction => {damage_reduction} (armor: {armor})");
        log::info!("Damage outcome: {damage_outcome}");
    }

    (damage_outcome, scaled_damage_outcome, false, attack_is_critical)
}

fn get_radius(ctx: &ReducerContext, entity_type: EntityType, entity_id: u64) -> f32 {
    if entity_type == EntityType::Player {
        return 0.5f32;
    } else if entity_type == EntityType::Enemy {
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(&entity_id) {
            let enemy_type = enemy.enemy_type as i32;
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();
            return enemy_desc.radius;
        } else {
            return 0.0;
        }
    }
    0.0
}
