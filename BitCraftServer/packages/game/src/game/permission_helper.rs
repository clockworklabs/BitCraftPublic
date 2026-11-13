use spacetimedb::{log, ReducerContext};

use crate::messages::{components::*, empire_shared::*, static_data::*};

use super::{claim_helper::get_claim_on_tile, dimensions, entities::building_state::BuildingState, game_state::game_state_filters};

use crate::game::coordinates::*;

pub fn can_interact_with_tile(ctx: &ReducerContext, coordinates: SmallHexTile, player_entity_id: u64, permission: ClaimPermission) -> bool {
    let tile_claim = get_claim_on_tile(ctx, coordinates);
    if tile_claim.is_none() {
        return true;
    }
    let tile_claim = tile_claim.unwrap();
    self::has_permission(ctx, player_entity_id, coordinates.dimension, tile_claim.claim_id, permission)
}

pub fn can_interact_with_building(
    ctx: &ReducerContext,
    building_state: &BuildingState,
    player_entity_id: u64,
    permission: ClaimPermission,
) -> bool {
    let building_entity_id = building_state.entity_id;
    let building_desc = ctx.db.building_desc().id().find(building_state.building_description_id).unwrap();
    let interaction_level = match permission {
        ClaimPermission::Usage => building_desc.interact_permission,
        ClaimPermission::Inventory => building_desc.interact_permission,
        _ => building_desc.build_permission,
    };

    match interaction_level {
        BuildingInteractionLevel::All => true,
        BuildingInteractionLevel::None => false,
        BuildingInteractionLevel::Empire => {
            let claim_desc = ctx.db.claim_state().entity_id().find(building_state.claim_entity_id);
            if let Some(ref claim) = claim_desc {
                if building_desc.has_category(ctx, BuildingCategory::EmpireFoundry) {
                    if let Some(empire_player_data_state) = ctx.db.empire_player_data_state().entity_id().find(player_entity_id) {
                        if let Some(empire_settlement_state) = ctx
                            .db
                            .empire_settlement_state()
                            .building_entity_id()
                            .find(claim.owner_building_entity_id)
                        {
                            if empire_player_data_state.empire_entity_id == empire_settlement_state.empire_entity_id {
                                return EmpirePlayerDataState::has_permission(ctx, player_entity_id, EmpirePermission::CraftHexiteCapsule)
                                    || EmpirePlayerDataState::has_permission(
                                        ctx,
                                        player_entity_id,
                                        EmpirePermission::CollectHexiteCapsule,
                                    );
                            }
                        }
                    }

                    return false;
                }

                if claim.has_co_owner_permissions(ctx, player_entity_id) {
                    // Empire Buildings are always accessible by Claim Owner(s)
                    return true;
                }
            } else {
                // Empire buildings outside a claim should be accessible by everyone
                return true;
            }

            let empire_node = match ctx.db.empire_node_state().entity_id().find(building_state.entity_id) {
                Some(en) => Some(en),
                None => {
                    if let Some(ref claim) = claim_desc {
                        ctx.db.empire_node_state().entity_id().find(claim.owner_building_entity_id)
                    } else {
                        None
                    }
                }
            };
            if let Some(empire_node) = empire_node {
                if permission == ClaimPermission::Usage && building_desc.has_category(ctx, BuildingCategory::Watchtower) {
                    // Any empire can somehow interact with a watchtower (start a siege, help with the siege even if their empire is not part of the conflict)
                    // Non-affiliated players can interact with a watchtower as well to join the empire
                    return true;
                }

                if let Some(player_rank) = ctx.db.empire_player_data_state().entity_id().find(player_entity_id) {
                    if permission == ClaimPermission::Build {
                        if empire_node.energy == 0 {
                            // Depleted watchtowers on empire influence can be destroyed by anyone part of that empire
                            if let Some(chunk) = ctx.db.empire_chunk_state().chunk_index().find(empire_node.chunk_index) {
                                if chunk.empire_entity_id == player_rank.empire_entity_id {
                                    return true;
                                }
                            }
                        }

                        // For now only allow the emperor of the node's empire to interact distructively with empire nodes that have > 0 energy
                        return player_rank.rank == 0 && player_rank.empire_entity_id == empire_node.empire_entity_id;
                    }

                    // Need to be part of the building's empire to interact with it
                    return empire_node.empire_entity_id == player_rank.empire_entity_id;
                }
            }

            // Using Building Empire
            if permission != ClaimPermission::Build {
                if !building_desc.has_category(ctx, BuildingCategory::Storage) {
                    log::error!("What kind of empire building is this? This is not a Hexite Capsule Reserve or a Watchtower");
                }

                let mut empire_settlement = ctx.db.empire_settlement_state().building_entity_id().find(building_state.entity_id);
                if empire_settlement.is_none() {
                    if let Some(ref claim) = claim_desc {
                        empire_settlement = ctx
                            .db
                            .empire_settlement_state()
                            .building_entity_id()
                            .find(claim.owner_building_entity_id);
                    }
                }
                if let Some(empire_settlement) = empire_settlement {
                    if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(player_entity_id) {
                        if rank.empire_entity_id == empire_settlement.empire_entity_id {
                            return EmpirePlayerDataState::has_permission(ctx, player_entity_id, EmpirePermission::CollectHexiteCapsule);
                        }
                    }
                }
            }
            false
        }
        _ => {
            if let Some(claim_desc) = ctx.db.claim_state().entity_id().find(building_state.claim_entity_id) {
                if claim_desc.neutral {
                    return false;
                }

                if claim_desc.local_state(ctx).supplies == 0 || claim_desc.owner_player_entity_id == 0 {
                    return true;
                }

                if claim_desc.owner_building_entity_id == building_entity_id
                    && player_entity_id != claim_desc.owner_player_entity_id
                    && permission == ClaimPermission::Build
                {
                    // only owner can destroy the claim
                    return false;
                }
            }
            self::has_permission(
                ctx,
                player_entity_id,
                game_state_filters::coordinates_any(ctx, building_entity_id).dimension,
                building_state.claim_entity_id,
                permission,
            )
        }
    }
}

pub fn has_permission(
    ctx: &ReducerContext,
    player_entity_id: u64,
    dimension: u32,
    claim_entity_id: u64,
    permission: ClaimPermission,
) -> bool {
    // Check rent permission for rented interiors
    if dimension != dimensions::OVERWORLD {
        let dimension_description_network = DimensionNetworkState::get(ctx, dimension).unwrap();
        // If it's rented, assert that the player is part of the whitelist. Claims don't matter.
        if dimension_description_network.rent_entity_id != 0 {
            let rent = ctx
                .db
                .rent_state()
                .entity_id()
                .find(dimension_description_network.rent_entity_id)
                .unwrap();
            return rent.white_list.contains(&player_entity_id);
        }
    }

    // If it's not rented, go on with claim permissions
    self::has_claim_permission(ctx, claim_entity_id, player_entity_id, permission)
}

pub fn has_claim_permission(ctx: &ReducerContext, claim_entity_id: u64, player_entity_id: u64, permission: ClaimPermission) -> bool {
    if let Some(claim_desc) = ctx.db.claim_state().entity_id().find(claim_entity_id) {
        // A neutral claim can be used and looted by anyone
        if claim_desc.neutral && (permission == ClaimPermission::Inventory || permission == ClaimPermission::Usage) {
            return true;
        }

        // A claim with no owner can be interacted with by anyone
        if !claim_desc.neutral && claim_desc.owner_player_entity_id == 0 {
            return true;
        }

        // A claim with no shield can be interacted with by anyone
        if !claim_desc.neutral && claim_desc.local_state(ctx).supplies == 0 {
            return true;
        }

        // Neutral claims can be used by everyone
        if let Some(building_state) = ctx.db.building_state().entity_id().find(claim_desc.owner_building_entity_id) {
            if let Some(building_claim) = ctx
                .db
                .building_claim_desc()
                .building_id()
                .find(building_state.building_description_id)
            {
                if building_claim.claim_type == ClaimType::Neutral && permission == ClaimPermission::Usage {
                    return true;
                }
            }
        }

        if let Some(member) = ctx
            .db
            .claim_member_state()
            .player_claim()
            .filter((player_entity_id, claim_entity_id))
            .next()
        {
            return match permission {
                ClaimPermission::Build => member.build_permission,
                ClaimPermission::Inventory => member.inventory_permission,
                ClaimPermission::Usage => true,
            };
        }

        return false;
    }
    true
}
