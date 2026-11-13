use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::game_state;
use crate::messages::components::{ability_state, action_bar_state, AbilityState, AbilityType, ActionBarState, ActionCooldown};

#[spacetimedb::reducer]
#[feature_gate]
pub fn ability_set(ctx: &ReducerContext, action_bar_index: u8, local_ability_index: u8, ability: AbilityType) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    reduce(ctx, actor_id, action_bar_index, local_ability_index as i8, ability)
}

pub fn reduce(
    ctx: &ReducerContext,
    actor_id: u64,
    action_bar_index: u8,
    local_ability_index: i8,
    ability: AbilityType,
) -> Result<(), String> {
    if action_bar_index > 3 {
        return Err("Invalid action bar index".into());
    }

    if local_ability_index > 11 {
        // Support 0 to 11 (12 slots)
        return Err("Invalid ability index".into());
    }

    if let AbilityType::_Unsupported(_) = ability { return Err("Invalid ability".into()) }

    let ability_entity_id;

    if let Some(player_ability) = ctx
        .db
        .ability_state()
        .owner_entity_id()
        .filter(actor_id)
        .find(|a| a.ability == ability)
    {
        ability_entity_id = player_ability.entity_id;
    } else {
        let ability = AbilityState {
            entity_id: game_state::create_entity(ctx),
            owner_entity_id: actor_id,
            ability,
            cooldown: ActionCooldown {
                timestamp: 0,
                cooldown: 0.0,
            },
        };
        ability_entity_id = ability.entity_id;
        ctx.db.ability_state().insert(ability);
    };

    // Unmapped auto-attacks, we don't need an action bar slot for those.
    if local_ability_index < 0 {
        // Everytime we set or remove an ability, we check to clean up expired unmapped abilities except from equipped weapons auto-attacks
        AbilityState::clean_up_unmapped_expired_abilities(ctx, actor_id);
        return Ok(());
    }
    let local_ability_index = local_ability_index as u8;

    let mut ability_slot = ctx
        .db
        .action_bar_state()
        .by_player_slot()
        .filter((actor_id, action_bar_index, local_ability_index))
        .next()
        .unwrap_or(ActionBarState {
            entity_id: game_state::create_entity(ctx),
            player_entity_id: actor_id,
            action_bar_index,
            local_ability_index,
            ability_entity_id,
        });

    ability_slot.ability_entity_id = ability_entity_id;
    ctx.db.action_bar_state().entity_id().insert_or_update(ability_slot);

    // Everytime we set or remove an ability, we check to clean up expired unmapped abilities except from equipped weapons auto-attacks
    AbilityState::clean_up_unmapped_expired_abilities(ctx, actor_id);

    Ok(())
}
