use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{
        action_request::PlayerPermissionEditRequest,
        components::{
            building_state, location_state, mobile_entity_state, permission_state, player_housing_state, Permission, PermissionState,
        },
    },
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn permission_edit(ctx: &ReducerContext, request: PlayerPermissionEditRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    PermissionState::validate_group(ctx, request.allowed_entity_id, request.group)?;

    let mut player_permission = None;

    if ctx.db.player_housing_state().entity_id().find(request.ordained_entity_id).is_some() {
        player_permission = PermissionState::get_permission_with_entity(ctx, actor_id, request.ordained_entity_id, None, None);
    } else if let Some(building) = ctx.db.building_state().entity_id().find(request.ordained_entity_id) {
        player_permission = PermissionState::get_player_permission_for_building(ctx, actor_id, &building);
    } else if let Some(location) = ctx.db.location_state().entity_id().find(request.ordained_entity_id) {
        player_permission = PermissionState::get_player_permission_for_tile(ctx, actor_id, location.coordinates().into());
    } else if let Some(mobile) = ctx.db.mobile_entity_state().entity_id().find(request.ordained_entity_id) {
        player_permission = PermissionState::get_player_permission_for_tile(ctx, actor_id, mobile.coordinates().into())
    }

    if let Some(permission) = player_permission {
        if permission == Permission::OverrideNoAccess {
            return Err("You don't have permission".into());
        }
        if (permission as i32) < (Permission::CoOwner as i32) {
            return Err("Only owners and co-owners can edit permissions".into());
        }

        if let Some(requested_permission) = request.permission {
            if requested_permission == Permission::Owner {
                return Err("Owner permission cannot be granted".into());
            }
            if requested_permission != Permission::OverrideNoAccess && (requested_permission as i32) >= (permission as i32) {
                return Err("You cannot grant a permission equal to or higher than your own".into());
            }
        }

        // Editing the player housing edits all dimensions within
        let ordained_entities = if let Some(player_housing) = ctx.db.player_housing_state().entity_id().find(request.ordained_entity_id) {
            player_housing.get_permission_entities(ctx)
        } else {
            vec![request.ordained_entity_id]
        };

        for ordained_entity_id in ordained_entities {
            if let Some(mut target_permission) = PermissionState::get(ctx, ordained_entity_id, request.allowed_entity_id) {
                if (permission as i32) <= target_permission.rank {
                    return Err("You cannot edit the permission of an entity that is of higher rank than you".into());
                }
                // edit existing permission
                if let Some(requested_rank) = request.permission {
                    // change the rank
                    target_permission.rank = requested_rank as i32;
                    ctx.db.permission_state().entity_id().update(target_permission);
                } else {
                    // delete the permission
                    ctx.db.permission_state().entity_id().delete(target_permission.entity_id);
                }
            } else {
                // new permission
                if request.permission.is_none() {
                    return Err("Already no permission".into());
                }

                let target_permission = PermissionState::new(
                    ctx,
                    ordained_entity_id,
                    request.allowed_entity_id,
                    request.group,
                    request.permission.unwrap(),
                )?;
                ctx.db.permission_state().insert(target_permission);
            }
        }
    } else {
        return Err("No permission to apply".into());
    }

    Ok(())
}
