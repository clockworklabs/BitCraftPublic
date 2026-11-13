use spacetimedb::ReducerContext;

use crate::{
    game::{
        claim_helper,
        game_state::{self, game_state_filters},
    },
    messages::{
        components::{
            claim_member_state, claim_state, dimension_description_state, permission_state, Permission, PermissionGroup, PermissionState,
        },
        empire_shared::{empire_player_data_state, empire_state},
    },
};

use super::{building_state::BuildingState, enemy_state::SmallHexTile};

impl PermissionState {
    pub fn new(
        ctx: &ReducerContext,
        ordained_entity_id: u64,
        allowed_entity_id: u64,
        group: PermissionGroup,
        rank: Permission,
    ) -> Result<Self, String> {
        if group == PermissionGroup::Everyone && rank == Permission::OverrideNoAccess {
            return Err("No Access can't be set to Everyone".into());
        }
        if rank == Permission::Owner && group != PermissionGroup::Player {
            return Err("Owner permission can only be assigned to a single player".into());
        }
        Ok(PermissionState {
            entity_id: game_state::create_entity(ctx),
            ordained_entity_id,
            allowed_entity_id,
            group: group as i32,
            rank: rank as i32,
        })
    }

    pub fn validate_group(ctx: &ReducerContext, allowed_entity_id: u64, group: PermissionGroup) -> Result<(), String> {
        match group {
            PermissionGroup::Player => Ok(()), //As we can add players on another region, we cannot validate this
            PermissionGroup::Claim => {
                if ctx.db.claim_state().entity_id().find(allowed_entity_id).is_some() {
                    Ok(())
                } else {
                    Err("Unknown Claim".into())
                }
            }
            PermissionGroup::Empire => {
                if ctx.db.empire_state().entity_id().find(allowed_entity_id).is_some() {
                    Ok(())
                } else {
                    Err("Unknown Empire".into())
                }
            }
            PermissionGroup::Everyone => Ok(()),
        }
    }

    pub fn get(ctx: &ReducerContext, ordained_entity_id: u64, allowed_entity_id: u64) -> Option<PermissionState> {
        ctx.db
            .permission_state()
            .ordained_and_allowed_entity_id()
            .filter((ordained_entity_id, allowed_entity_id))
            .next()
    }

    fn found_permission(ctx: &ReducerContext, target_entity_id: u64, available_entities: &[u64]) -> Option<i32> {
        let mut permissions: Vec<PermissionState> = ctx.db.permission_state().ordained_entity_id().filter(target_entity_id).collect();
        permissions.sort_by_key(|p1| p1.group);
        let p = permissions
            .iter()
            .find(|p| p.group == PermissionGroup::Everyone as i32 || available_entities.contains(&p.allowed_entity_id));
        p.map(|permission| permission.rank)
    }

    pub fn can_interact_with_building(
        ctx: &ReducerContext,
        player_entity_id: u64,
        building: &BuildingState,
        permission: Permission,
    ) -> bool {
        if let Some(permission_found) = Self::get_player_permission_for_building(ctx, player_entity_id, building) {
            permission_found.meets(permission)
        } else {
            // can interact if no permission at all is found
            true
        }
    }

    pub fn get_player_permission_for_building(ctx: &ReducerContext, player_entity_id: u64, building: &BuildingState) -> Option<Permission> {
        let location = game_state_filters::coordinates_any(ctx, building.entity_id);
        let dimension_id = location.dimension;
        let claim_entity_id = if building.claim_entity_id != 0 {
            Some(building.claim_entity_id)
        } else {
            None
        };
        Self::get_permission_with_entity(ctx, player_entity_id, building.entity_id, Some(dimension_id), claim_entity_id)
    }

    pub fn can_interact_with_tile(ctx: &ReducerContext, player_entity_id: u64, location: SmallHexTile, permission: Permission) -> bool {
        if let Some(permission_found) = Self::get_player_permission_for_tile(ctx, player_entity_id, location) {
            permission_found.meets(permission)
        } else {
            // can interact if no permission at all is found
            true
        }
    }

    pub fn get_player_permission_for_tile(ctx: &ReducerContext, player_entity_id: u64, location: SmallHexTile) -> Option<Permission> {
        let dimension_id = location.dimension;
        let mut claim_entity_id = None;
        if let Some(claimed_tile) = claim_helper::get_claim_on_tile(ctx, location) {
            claim_entity_id = Some(claimed_tile.claim_id);
        }
        Self::get_permission_with_entity(ctx, player_entity_id, 0, Some(dimension_id), claim_entity_id)
    }

    pub fn get_permission_with_entity(
        ctx: &ReducerContext,
        player_entity_id: u64,
        target_entity_id: u64,
        dimension_id: Option<u32>,
        claim_entity_id: Option<u64>,
    ) -> Option<Permission> {
        // `available_entities` is a list of `entity_id`s
        // for claims empires which the player `player_entity_id` is a member of,
        // plus the player's own `entity_id` (`player_entity_id`).
        let mut available_entities = vec![player_entity_id];

        // Get all the claims that `player_entity_id` is a member of.
        available_entities.extend(
            ctx.db
                .claim_member_state()
                .player_entity_id()
                .filter(player_entity_id)
                // We assume that `claim_member_state.claim_entity_id` is a foreign key,
                // and therefore that the corresponding `claim_state` row exists without checking.
                .map(|claim_member_state| claim_member_state.claim_entity_id),
        );

        // Finally, add the empire which the player is a member of, if any.
        if let Some(player_empire) = ctx.db.empire_player_data_state().entity_id().find(player_entity_id) {
            available_entities.push(player_empire.empire_entity_id);
        }

        // We can't return the permission right away in case it's blocked by another criteria
        let mut permission_found = None;

        // First find out what permissions apply to the entity
        if let Some(p) = Self::found_permission(ctx, target_entity_id, &available_entities) {
            if p == Permission::OverrideNoAccess as i32 {
                return Some(Permission::to_enum(p));
            }
            permission_found = Some(Permission::to_enum(p));
        }

        // Next find out what permissions apply to the interior
        if let Some(dimension) = dimension_id {
            if let Some(interior) = ctx.db.dimension_description_state().dimension_id().find(dimension) {
                if let Some(p) = Self::found_permission(ctx, interior.entity_id, &available_entities) {
                    if p == Permission::OverrideNoAccess as i32 {
                        return Some(Permission::to_enum(p));
                    }
                    if permission_found.is_none() {
                        permission_found = Some(Permission::to_enum(p));
                    }
                }
            }
        }

        // Next find out what permissions apply to the claim
        if let Some(claim_entity_id) = claim_entity_id {
            if let Some(p) = Self::found_permission(ctx, claim_entity_id, &available_entities) {
                if p == Permission::OverrideNoAccess as i32 {
                    return Some(Permission::to_enum(p));
                }
                if permission_found.is_none() {
                    permission_found = Some(Permission::to_enum(p));
                }
            }
        }

        permission_found
    }
}
