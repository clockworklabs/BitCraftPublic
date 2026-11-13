use spacetimedb::ReducerContext;

use crate::{
    game::{coordinates::SmallHexTile, dimensions, game_state::game_state_filters},
    messages::{
        components::{building_state, claim_state},
        empire_shared::*,
        static_data::parameters_desc,
    },
    unwrap_or_err,
};

impl EmpirePlayerDataState {
    pub fn has_permission(ctx: &ReducerContext, player_entity_id: u64, permission: EmpirePermission) -> bool {
        if let Some(rank) = EmpireRankState::get_player_rank(ctx, player_entity_id) {
            let i = permission as usize;
            return match rank.permissions.get(i) {
                Some(v) => *v,
                None => false,
            };
        }
        false
    }
}

impl EmpireState {
    pub fn get_building_empire_entity_id(ctx: &ReducerContext, building_entity_id: u64) -> u64 {
        // This will take care of aligned claims and watchtowers
        if let Some(empire_node) = ctx.db.empire_node_state().entity_id().find(building_entity_id) {
            return empire_node.empire_entity_id;
        }
        if let Some(building) = ctx.db.building_state().entity_id().find(building_entity_id) {
            if let Some(claim) = ctx.db.claim_state().entity_id().find(building.claim_entity_id) {
                if let Some(empire_settlement_state) = ctx.db.empire_settlement_state().claim_entity_id().find(claim.entity_id) {
                    return empire_settlement_state.empire_entity_id;
                }
            }
        }
        0
    }
}

impl EmpireRankState {
    pub fn get_rank(ctx: &ReducerContext, empire_entity_id: u64, rank: u8) -> Option<EmpireRankState> {
        ctx.db.empire_rank_state().empire_rank().filter((empire_entity_id, rank)).next()
    }

    pub fn get_player_rank(ctx: &ReducerContext, player_entity_id: u64) -> Option<EmpireRankState> {
        if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(player_entity_id) {
            return Self::get_rank(ctx, rank.empire_entity_id, rank.rank);
        }
        None
    }
}

impl EmpireNodeSiegeState {
    pub fn get(ctx: &ReducerContext, building_entity_id: u64, empire_entity_id: u64) -> Option<EmpireNodeSiegeState> {
        let mut sieges_on_node = ctx.db.empire_node_siege_state().building_entity_id().filter(building_entity_id);
        sieges_on_node.find(|n| n.empire_entity_id == empire_entity_id)
    }

    pub fn has_active_siege(ctx: &ReducerContext, building_entity_id: u64) -> bool {
        ctx.db
            .empire_node_siege_state()
            .building_entity_id()
            .filter(building_entity_id)
            .any(|s| s.active)
    }

    pub fn validate_action(ctx: &ReducerContext, actor_id: u64, building_entity_id: u64) -> Result<(), String> {
        if ctx.db.empire_node_state().entity_id().find(building_entity_id).is_none() {
            return Err("This building cannot be sieged".into());
        }
        if ctx.db.empire_player_data_state().entity_id().find(actor_id).is_none() {
            return Err("You are not a member of an empire".into());
        }
        Ok(())
    }

    pub fn validate_add_supplies(
        ctx: &ReducerContext,
        player_entity_id: u64,
        building_entity_id: u64,
        proxy_empire_entity_id: Option<u64>,
    ) -> Result<(), String> {
        // Add supplies to ongoing siege, either for attacker or defender. We cannot start a new siege here.
        if !EmpireNodeSiegeState::has_active_siege(ctx, building_entity_id) {
            return Err("Cannot add siege supplies when there is no siege".into());
        }

        let rank = ctx.db.empire_player_data_state().entity_id().find(player_entity_id).unwrap();
        let mut siege = EmpireNodeSiegeState::get(ctx, building_entity_id, rank.empire_entity_id);

        let participating = match siege.as_ref() {
            Some(s) => s.active,
            None => false,
        };

        if !participating {
            // There is no aligned empire participating in the current active siege, but maybe the player is helping a valid empire?
            if let Some(proxy_empire_entity_id) = proxy_empire_entity_id {
                siege = EmpireNodeSiegeState::get(ctx, building_entity_id, proxy_empire_entity_id);
                if siege.is_none() {
                    return Err("You cannot drop supplies for an empire that is not participating in the current siege".into());
                }
            } else {
                return Err("You cannot start a new siege while this watchtower is already sieged".into());
            }
        }

        Ok(())
    }
}

impl EmpireSettlementState {
    pub fn from_claim(ctx: &ReducerContext, claim_entity_id: u64) -> Option<EmpireSettlementState> {
        ctx.db.empire_settlement_state().claim_entity_id().find(claim_entity_id)
    }
}

impl EmpireNodeState {
    // Check if the node can activate and project influence:
    // - No other node is active in the current chunk
    // - No different empire controls this chunk
    pub fn validate_influence(ctx: &ReducerContext, chunk_index: u64, empire_entity_id: u64) -> Result<(), String> {
        if ctx.db.empire_node_state().chunk_index().filter(chunk_index).any(|node| node.active) {
            return Err("Only one active empire node is allowed per chunk".into());
        }

        if let Some(empire_chunk) = ctx.db.empire_chunk_state().chunk_index().find(chunk_index) {
            if empire_chunk.empire_entity_id != 0 && empire_chunk.empire_entity_id != empire_entity_id {
                return Err("Another empire controls this territory".into());
            }
        }

        Ok(())
    }
}

pub fn empire_resupply_node_validate(ctx: &ReducerContext, actor_id: u64, building_entity_id: u64) -> Result<(), String> {
    if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::SupplyNode) {
        return Err("You don't have the permissions to resupply a node".into());
    }

    // Find the player's empire affiliation
    let player_rank = unwrap_or_err!(
        ctx.db.empire_player_data_state().entity_id().find(actor_id),
        "You need to be part of an empire to deposit a hexite capsule"
    );

    // Find the building empire affiliation
    let empire_node = unwrap_or_err!(
        ctx.db.empire_node_state().entity_id().find(building_entity_id),
        "The building needs to be part of an empire to receive a hexite capsule"
    );

    if player_rank.empire_entity_id != empire_node.empire_entity_id {
        return Err("You need to be part of the empire to supply this empire node.".into());
    }

    if let Some(empire_chunk) = ctx.db.empire_chunk_state().chunk_index().find(empire_node.chunk_index) {
        if empire_chunk.empire_entity_id != 0 && empire_chunk.empire_entity_id != empire_node.empire_entity_id {
            return Err("Cannot resupply watchtower when not under your empire control".into());
        }
    }

    if EmpireNodeSiegeState::get(ctx, building_entity_id, empire_node.empire_entity_id).is_some() {
        return Err("Cannot resupply watchtower directly when it's under siege".into());
    }

    Ok(())
}

pub fn validate_siege_distance(
    ctx: &ReducerContext,
    building_entity_id: u64,
    coord: SmallHexTile,
    error_message: &str,
    padding: i32,
) -> Result<(), String> {
    let watchtower_location = game_state_filters::coordinates(ctx, building_entity_id);
    let distance = watchtower_location.distance_to(coord);
    let params = ctx.db.parameters_desc().version().find(0).unwrap();
    let min = params.empire_min_siege_distance - padding;
    let max = params.empire_max_siege_distance + padding;
    if distance < min || distance > max {
        return Err(format!("{{0}} within {{1}} and {{2}} tiles from the target watchtower|~{error_message}|~{min}|~{max}"));
    }

    Ok(())
}

// not a reducer since we go through the project site placement
pub fn validate_empire_build_foundry(ctx: &ReducerContext, actor_id: u64, coord: SmallHexTile) -> Result<(), String> {
    if coord.dimension != dimensions::OVERWORLD {
        return Err("Cannot build an empire foundry indoors".into());
    }

    if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::CraftHexiteCapsule) {
        return Err("You don't have the permissions to build an empire foundry".into());
    }

    Ok(())
}
