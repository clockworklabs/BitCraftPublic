use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        discovery::Discovery,
        game_state::{self, game_state_filters},
        reducer_helpers::deployable_helpers,
    },
    messages::{action_request::PlayerCollectibleActivateRequest, components::*, empire_shared::empire_player_data_state, static_data::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn collectible_activate(ctx: &ReducerContext, request: PlayerCollectibleActivateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.vault_index, request.activated, false)
}

fn get_deployable_deploybale_appearance_override(
    ctx: &ReducerContext,
    collectible_id: i32,
) -> Result<DeployableAppearanceOverrideDesc, String> {
    ctx.db
        .deployable_appearance_override_desc()
        .collectible_id()
        .find(&collectible_id)
        .ok_or_else(|| format!("Missing deployable appearance override static data for collectible {{0}}.|~{collectible_id}"))
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, vault_index: i32, activated: bool, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    // DAB Note: vault_index should probably be a usize or u16. No reason to use a i32.
    if vault_index < 0 {
        return Err("Invalid collectible, please fill a bug report with a screenshot of the collectible in your vault".into());
    }

    let mut vault = unwrap_or_err!(ctx.db.vault_state().entity_id().find(&actor_id), "Vault not initialized");
    let collectibles = &mut vault.collectibles;
    if collectibles.len() as i32 <= vault_index {
        return Err("Vault does not contain this collectible.".into());
    }

    let collectible_id = unwrap_or_err!(collectibles.get(vault_index as usize), "You no longer own this collectible").id; // this shouldn't ever error
    let collectible_desc = unwrap_or_err!(ctx.db.collectible_desc().id().find(&collectible_id), "Invalid collectible type.");

    if collectible_desc.collectible_type == CollectibleType::Emote {
        return Err("Cannot activate emotes".into());
    }

    if collectible_desc.collectible_type == CollectibleType::ClothesCape && activated {
        if ctx.db.empire_player_data_state().entity_id().find(actor_id).is_none() {
            return Err("Cannot equip an empire cloak when not part of an empire".into());
        }
    }

    for knowledge_req in &collectible_desc.required_knowledges_to_use {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *knowledge_req) {
            return Err("You don't meet the knowledge requirements to use this collectible".into());
        }
    }

    let deploybale_appearance_override = if collectible_desc.collectible_type == CollectibleType::DeployableAppearanceOverride {
        Some(get_deployable_deploybale_appearance_override(ctx, collectible_id)?)
    } else {
        None
    };

    let collectible_is_activated = collectibles
        .get(vault_index as usize)
        .map(|collectible| collectible.activated)
        .unwrap_or(false);

    if collectible_is_activated != activated {
        if let Some(deploybale_appearance_override) = &deploybale_appearance_override {
            if deploybale_appearance_override.affected_models().iter().any(|model_address| {
                deployable_helpers::has_deployed_deployable_with_model_address(ctx, actor_id, model_address)
            }) {
                return Err("Cannot change the appearance of a deployable while it's deployed, store it first.".into());
            }
        }
    }

    let max_count = if collectible_desc.collectible_type == CollectibleType::DeployableAppearanceOverride {
        i32::MAX
    } else {
        collectible_desc.max_equip_count
    };

    if max_count == 0 {
        return Err("Cannot activate this collectible.".into());
    }

    // make sure nothing equipped invalidates this type
    for i in 0..collectibles.len() {
        let col = &collectibles[i];
        if col.activated {
            let col_desc = unwrap_or_err!(ctx.db.collectible_desc().id().find(&col.id), "Corrupted vault inventory.");
            if col_desc.invalidates_type == collectible_desc.collectible_type {
                return Err("Something you are wearing is blocking that slot".into());
            }
        }
    }

    let mut count = 0;

    for i in 0..collectibles.len() {
        let col = &mut collectibles[i];
        if i as i32 == vault_index {
            col.activated = activated;

            if collectible_desc.collectible_type == CollectibleType::HousingFloor
                || collectible_desc.collectible_type == CollectibleType::HousingWalls
            {
                let dimension = game_state_filters::coordinates_any(ctx, actor_id).dimension;
                if let Some(current_housing) = PlayerHousingState::from_dimension(ctx, dimension) {
                    if current_housing.entity_id != actor_id {
                        return Err("You must be inside your house to activate or deactivate housing collectibles".into());
                    }
                } else {
                    return Err("You must be inside your house to activate or deactivate housing collectibles".into());
                }
                let mut housing_customization = ctx.db.player_housing_customization_state().entity_id().find(actor_id).unwrap();
                if collectible_desc.collectible_type == CollectibleType::HousingFloor {
                    housing_customization.floor_collectible_id = if activated { col.id } else { 0 };
                } else {
                    housing_customization.wall_collectible_id = if activated { col.id } else { 0 };
                }
                ctx.db
                    .player_housing_customization_state()
                    .entity_id()
                    .update(housing_customization);
            }

            if activated {
                count += 1;

                // potentially unequip other collectibles
                if collectible_desc.invalidates_type != CollectibleType::Default {
                    for j in 0..collectibles.len() {
                        let other_col = &mut collectibles[j];
                        if other_col.activated {
                            let other_collectible_desc =
                                unwrap_or_err!(ctx.db.collectible_desc().id().find(&other_col.id), "Corrupted vault inventory.");
                            if other_collectible_desc.collectible_type == collectible_desc.invalidates_type {
                                other_col.activated = false;
                            }
                        }
                    }
                }

                if let Some(deploybale_appearance_override) = &deploybale_appearance_override {
                    for j in 0..collectibles.len() {
                        if j == i {
                            continue;
                        }

                        let other_col = &mut collectibles[j];
                        if !other_col.activated {
                            continue;
                        }

                        let other_collectible_desc =
                            unwrap_or_err!(ctx.db.collectible_desc().id().find(&other_col.id), "Corrupted vault inventory.");
                        if other_collectible_desc.collectible_type != CollectibleType::DeployableAppearanceOverride {
                            continue;
                        }

                        let other = get_deployable_deploybale_appearance_override(ctx, other_col.id)?;
                        if other.affects_any_same_model_as(deploybale_appearance_override) {
                            other_col.activated = false;
                        }
                    }
                }
            }
        } else {
            let col_desc = unwrap_or_err!(ctx.db.collectible_desc().id().find(&col.id), "Corrupted vault inventory.");
            if activated && col_desc.collectible_type == collectible_desc.collectible_type {
                if max_count == 1 {
                    // shortcut - for limit of 1 activated collectible, unequip all other similar collectibles
                    col.activated = false;
                } else {
                    // for a limit over 1, player might need to deactivate some collectibles first
                    count += 1;
                }
            }
        }
    }

    if count > max_count {
        return Err(format!("Cannot activate more than {{0}} collectibles of this kind.|~{}", max_count).into());
    }

    if !dry_run {
        ctx.db.vault_state().entity_id().update(vault);
    }

    Ok(())
}
