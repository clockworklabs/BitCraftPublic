use spacetimedb::{ReducerContext, Table, Timestamp};

use crate::{
    game::game_state,
    messages::{
        components::{ability_state, action_bar_state, combat_state, AbilityType, ActionBarState, PlayerState},
        static_data::{combat_action_desc_v3, weapon_desc, CombatActionDescV3},
    },
    AbilityState, ActionCooldown,
};

impl AbilityState {
    pub fn new(ctx: &ReducerContext, owner_entity_id: u64, ability: AbilityType) -> AbilityState {
        let entity_id = game_state::create_entity(ctx);
        AbilityState {
            entity_id,
            owner_entity_id,
            ability,
            cooldown: ActionCooldown {
                timestamp: 0,
                cooldown: 0.0,
            },
        }
    }

    pub fn get(ctx: &ReducerContext, actor_id: u64, ability: AbilityType) -> AbilityState {
        match ctx
            .db
            .ability_state()
            .owner_entity_id()
            .filter(actor_id)
            .find(|a| a.ability == ability)
        {
            Some(a) => a,
            None => AbilityState::new(ctx, actor_id, ability),
        }
    }

    pub fn is_under_cooldown(&self, ctx: &ReducerContext, check_global: bool) -> bool {
        let now = game_state::unix_ms(ctx.timestamp);
        let availability = self.cooldown.timestamp + (self.cooldown.cooldown * 1000.0) as u64;
        if now < availability {
            // Not available yet
            return true;
        }

        if check_global {
            if let Some(combat) = ctx.db.combat_state().entity_id().find(self.owner_entity_id) {
                if let Some(global_cooldown) = combat.global_cooldown {
                    return now < global_cooldown.timestamp + (global_cooldown.cooldown * 1000.0) as u64;
                }
            }
        }
        false
    }

    pub fn set_cooldown(mut self, ctx: &ReducerContext, cooldown: f32, global_cooldown: f32, check_global: bool) -> bool {
        if self.is_under_cooldown(ctx, check_global) {
            // cannot set cooldown if already under cooldown
            return false;
        }
        let now = game_state::unix_ms(ctx.timestamp);
        if global_cooldown > 0.0 {
            if let Some(mut combat) = ctx.db.combat_state().entity_id().find(self.owner_entity_id) {
                combat.global_cooldown = Some(ActionCooldown {
                    timestamp: now,
                    cooldown: global_cooldown,
                });
                ctx.db.combat_state().entity_id().update(combat);
            }
        }

        self.cooldown = ActionCooldown { timestamp: now, cooldown };
        self.update(ctx);
        true
    }

    pub fn update(self, ctx: &ReducerContext) {
        ctx.db.ability_state().entity_id().insert_or_update(self);
    }

    pub fn clean_up_unmapped_expired_abilities(ctx: &ReducerContext, player_entity_id: u64) {
        let action_bar: Vec<ActionBarState> = ctx.db.action_bar_state().player_entity_id().filter(player_entity_id).collect();
        let hunting_action_id = if let Some(hunting_weapon) = PlayerState::get_hunting_weapon(ctx, player_entity_id) {
            let weapon_type = ctx.db.weapon_desc().item_id().find(hunting_weapon.item_id).unwrap().weapon_type;
            if let Some(action) = ctx
                .db
                .combat_action_desc_v3()
                .iter()
                .find(|a| a.auto_cast && a.weapon_type_requirements.contains(&weapon_type))
            {
                action.id
            } else {
                0
            }
        } else {
            0
        };
        let combat_action_id = if let Some(combat_weapon) = PlayerState::get_combat_weapon(ctx, player_entity_id) {
            if let None = ctx.db.weapon_desc().item_id().find(combat_weapon.item_id) {
                spacetimedb::log::info!(
                    "Player {} has invalid weapon id {} in combat slot",
                    player_entity_id,
                    combat_weapon.item_id
                );
                return;
            }
            let weapon_type = ctx.db.weapon_desc().item_id().find(combat_weapon.item_id).unwrap().weapon_type;
            if let Some(action) = ctx
                .db
                .combat_action_desc_v3()
                .iter()
                .find(|a| a.auto_cast && a.weapon_type_requirements.contains(&weapon_type))
            {
                action.id
            } else {
                0
            }
        } else {
            0
        };

        for ability_state in ctx.db.ability_state().owner_entity_id().filter(player_entity_id) {
            if action_bar
                .iter()
                .find(|ab| ab.ability_entity_id == ability_state.entity_id)
                .is_some()
            {
                continue;
            }
            // orphaned ability
            let availability = ability_state.cooldown.timestamp + (ability_state.cooldown.cooldown * 1000.0) as u64;
            let now = game_state::unix_ms(ctx.timestamp);
            if now >= availability {
                // Unless it's the current equipped weapon auto-attack, we need to get rid of this ability
                let keep = match ability_state.ability {
                    AbilityType::CombatAction(id) => id == hunting_action_id || id == combat_action_id,
                    _ => false,
                };
                if keep {
                    continue;
                }
                // Expired unmapped action that's not the currently equipped weapon / hunting weapon auto-attack... destroy
                ctx.db.ability_state().entity_id().delete(ability_state.entity_id);
            }
        }
    }

    pub fn set_combat_action_cooldown(
        &mut self,
        combat_action: &CombatActionDescV3,
        cooldown_multiplier: f32,
        weapon_cooldown_multiplier: f32,
        timestamp: Timestamp,
        include_lead_in: bool,
    ) {
        let raw_cooldown = combat_action.cooldown * weapon_cooldown_multiplier;
        let post_lead_in_cooldown = raw_cooldown - combat_action.lead_in_time;
        // We need to take off the lead-in time from the equation if this resolves during the attack segment, since the attack reducer is called after the lead-in
        // (We can't apply the cooldowns in the start of the attack animation since the action can still be cancelled by moving until it lands)
        let mut modified_cooldown = if include_lead_in { combat_action.lead_in_time } else { 0.0 };
        modified_cooldown += post_lead_in_cooldown / cooldown_multiplier;

        self.cooldown = ActionCooldown {
            timestamp: game_state::unix_ms(timestamp),
            cooldown: modified_cooldown,
        };
    }
}
