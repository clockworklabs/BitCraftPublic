use std::collections::HashSet;

use queues::*;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::game_state::{self, unix},
    inter_module::send_inter_module_message,
    location_state,
    messages::{
        components::{building_nickname_state, claim_member_state, user_state},
        empire_schema::*,
        empire_shared::*,
        generic::world_region_state,
        global::user_region_state,
        inter_module::{
            EmpireRemoveCrownMsg, EmpireUpdateEmperorCrownMsg, OnPlayerJoinedEmpireMsg, OnPlayerLeftEmpireMsg, RegionDestroySiegeEngineMsg,
        },
        static_data::EmpireNotificationType,
    },
    parameters_desc, signed_in_player_state, unwrap_or_err, unwrap_or_return, ChunkCoordinates, ParametersDesc, SmallHexTile,
    TerrainChunkState,
};

use super::empires::delete_empire_building;
use super::npc_empire::is_npc_empire;

impl EmpireNodeState {
    pub fn chunk_coordinates(&self, ctx: &ReducerContext) -> Option<ChunkCoordinates> {
        if let Some(claim_building_location) = ctx.db.location_state().entity_id().find(&self.entity_id) {
            return Some(claim_building_location.coordinates().chunk_coordinates());
        }
        None
    }

    // Empire Nodes are watchtowers only and created when the building is created. They can't be unaligned.
    // Claims are now Empire Settlements.
    pub fn new(watchtower_entity_id: u64, empire_entity_id: u64, coord: SmallHexTile) -> EmpireNodeState {
        let chunk_coordinates = coord.chunk_coordinates();
        let chunk_index = TerrainChunkState::chunk_index_from_coords(&chunk_coordinates);

        EmpireNodeState {
            entity_id: watchtower_entity_id,
            empire_entity_id,
            chunk_index,
            energy: 0,
            active: false,
            upkeep: 0,
            location: coord.into(),
        }
    }

    pub fn add_energy(&mut self, ctx: &ReducerContext, energy: i32, params: Option<ParametersDesc>) -> bool {
        let params = params.unwrap_or_else(|| ctx.db.parameters_desc().version().find(&0).unwrap());
        if self.energy >= params.empire_node_max_energy {
            return false;
        }
        self.energy = (self.energy + energy).min(params.empire_node_max_energy);
        true
    }

    // Activate an empire node by providing it energy (via a hexite capsule), possibly an empire and possibly setting it as a capital city
    // An empire node without supplies becomes deactivated but remains its affiliation
    pub fn activate(&mut self, ctx: &ReducerContext, supplies: i32) -> Result<(), String> {
        if self.active {
            return Err("Already active".into());
        }

        if supplies == 0 && self.energy == 0 {
            return Err("Can't active a node with no supplies".into());
        }

        if ctx
            .db
            .empire_node_state()
            .chunk_index()
            .filter(self.chunk_index)
            .any(|node| node.active)
        {
            return Err("Only one empire node can be active in a given chunk".into());
        }

        Self::validate_influence(ctx, self.chunk_index, self.empire_entity_id)?;

        // Stamp influence on surrounding chunks
        let updated_empires = self.stamp_influence(ctx);
        for updated_empire in updated_empires {
            EmpireState::update_crown_status(ctx, updated_empire);
        }

        self.add_energy(ctx, supplies, None);
        self.active = true;

        Ok(())
    }

    // Change ownership of an EmpireNode.
    pub fn convert(&mut self, ctx: &ReducerContext, supplies: i32, new_empire_entity_id: u64) {
        let mut updated_empires = self.unstamp_influence(ctx);

        self.empire_entity_id = new_empire_entity_id;

        // Stamp influence on surrounding chunks
        self.add_energy(ctx, supplies, None);
        self.active = true;

        if self.energy > 0 {
            let more_updated_empires = self.stamp_influence(ctx);
            for updated_empire in more_updated_empires {
                updated_empires.insert(updated_empire);
            }
        }

        for updated_empire in updated_empires {
            EmpireState::update_crown_status(ctx, updated_empire);
        }
    }

    // Deactivate the influence, but stay in the empire.
    // This can happen if out of supplies
    pub fn deactivate(&mut self, ctx: &ReducerContext) {
        if self.active {
            self.active = false;
            let updated_empires = self.unstamp_influence(ctx);
            for updated_empire in updated_empires {
                EmpireState::update_crown_status(ctx, updated_empire);
            }
        }
    }

    pub fn delete(self, ctx: &ReducerContext) {
        if self.active {
            let updated_empires = self.unstamp_influence(ctx);
            for updated_empire in updated_empires {
                EmpireState::update_crown_status(ctx, updated_empire);
            }
        }

        // Delete chunks assigned to this watchtower
        let chunks: Vec<EmpireChunkState> = ctx.db.empire_chunk_state().watchtower_entity_id().filter(self.entity_id).collect();
        for chunk in chunks {
            EmpireChunkState::delete_shared(ctx, chunk, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // End all ongoing sieges for this building
        for siege in ctx.db.empire_node_siege_state().building_entity_id().filter(self.entity_id) {
            siege.cancel_siege(ctx);
        }

        EmpireNodeState::delete_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    // Stamps the empire influence on all chunks assigned to this watchtower.
    // Chunks are pre-created during world gen with watchtower_entity_id set.
    // Returns a list of all empires whose territory size changed (due to newly acquired chunk)
    pub fn stamp_influence(&self, ctx: &ReducerContext) -> HashSet<u64> {
        let mut set = HashSet::new();

        let chunks: Vec<EmpireChunkState> = ctx.db.empire_chunk_state().watchtower_entity_id().filter(self.entity_id).collect();

        for empire_chunk in chunks {
            if empire_chunk.empire_entity_id == 0 {
                // Chunk has no current owner - this empire is claiming it
                empire_chunk.stamp_and_commit(ctx, self.empire_entity_id);
                set.insert(self.empire_entity_id); // this empire now controls this chunk (+1)
            } else if empire_chunk.empire_entity_id != self.empire_entity_id {
                // Chunk owned by different empire - should not happen with non-overlapping watchtowers
                log::warn!(
                    "Chunk {} already owned by empire {} when stamping for empire {}",
                    empire_chunk.chunk_index,
                    empire_chunk.empire_entity_id,
                    self.empire_entity_id
                );
                let previous_owner = empire_chunk.empire_entity_id;
                empire_chunk.stamp_and_commit(ctx, self.empire_entity_id);
                set.insert(previous_owner);
                set.insert(self.empire_entity_id);
            }
            // else: already owned by same empire, no-op
        }
        set
    }

    // Unstamp the empire influence from all chunks assigned to this watchtower.
    // Returns a list of all empires whose territory size changed
    pub fn unstamp_influence(&self, ctx: &ReducerContext) -> HashSet<u64> {
        let mut set = HashSet::new();

        let chunks: Vec<EmpireChunkState> = ctx.db.empire_chunk_state().watchtower_entity_id().filter(self.entity_id).collect();

        for empire_chunk in chunks {
            if empire_chunk.empire_entity_id == self.empire_entity_id {
                empire_chunk.unstamp_and_commit(ctx, self.empire_entity_id);
                set.insert(self.empire_entity_id); // this empire no longer controls this chunk (-1)
            }
        }
        set
    }
}

impl EmpireState {
    pub fn delete(&self, ctx: &ReducerContext) {
        let mut emperor_entity_id = 0;

        if let Some(emperor) = ctx
            .db
            .empire_player_data_state()
            .empire_entity_id()
            .filter(self.entity_id)
            .filter(|data| data.rank == 0)
            .next()
        {
            emperor_entity_id = emperor.entity_id;
        }

        // Delete foundries
        for foundry in ctx.db.empire_foundry_state().empire_entity_id().filter(self.entity_id) {
            delete_empire_building(ctx, 0, foundry.entity_id, false);
        }

        // End all ongoing sieges for this empire
        for siege in ctx.db.empire_node_siege_state().empire_entity_id().filter(self.entity_id) {
            siege.cancel_siege(ctx);
        }

        // All aligned node will become unaligned
        for empire_node in ctx.db.empire_node_state().empire_entity_id().filter(self.entity_id) {
            let empire_node_entity_id = empire_node.entity_id;
            // Watchtowers need to be destroyed
            //let building = ctx.db.building_state().entity_id().find(&empire_node_entity_id).unwrap();
            //let building_desc = ctx.db.building_desc().id().find(&building.building_description_id).unwrap();
            //if building_desc.has_category(ctx, crate::BuildingCategory::Watchtower) {
            delete_empire_building(ctx, 0, empire_node_entity_id, false);
            //} else {
            //    empire_node.empire_entity_id = 0;
            //    empire_node.active = false;
            //    EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
            //}
        }

        // All aligned claims will become unaligned
        for mut settlement in ctx.db.empire_settlement_state().empire_entity_id().filter(self.entity_id) {
            settlement.empire_entity_id = 0;
            EmpireSettlementState::update_shared(ctx, settlement, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // All aligned EmpireChunkStates will become unaligned
        for empire_chunk in ctx.db.empire_chunk_state().iter() {
            empire_chunk.unstamp_all_and_commit(ctx, self.entity_id);
        }

        // All aligned player ranks must be deleted
        let region = ctx.db.user_region_state().identity().find(ctx.sender).unwrap().region_id;
        for rank in ctx.db.empire_player_data_state().empire_entity_id().filter(self.entity_id) {
            let rank_entity_id = rank.entity_id;
            EmpirePlayerDataState::delete_shared(ctx, rank, crate::inter_module::InterModuleDestination::AllOtherRegions);
            ctx.db.empire_player_log_state().entity_id().delete(&rank_entity_id);

            send_inter_module_message(
                ctx,
                crate::messages::inter_module::MessageContentsV5::OnPlayerLeftEmpire(OnPlayerLeftEmpireMsg {
                    player_entity_id: rank_entity_id,
                    empire_entity_id: self.entity_id,
                }),
                crate::inter_module::InterModuleDestination::Region(region),
            );
        }

        // All ranks for this empire need to disappear
        for rank in ctx.db.empire_rank_state().empire_entity_id().filter(self.entity_id) {
            EmpireRankState::delete_shared(ctx, rank, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // Delete this empire's notifications
        for notification_entity_id in ctx
            .db
            .empire_notification_state()
            .empire_entity_id()
            .filter(self.entity_id)
            .map(|n| n.entity_id)
        {
            ctx.db.empire_notification_state().entity_id().delete(&notification_entity_id);
        }

        // Remove emperor crown collectible if the emperor exists (a merged empire no longer has an emperor)
        if emperor_entity_id != 0 {
            Self::remove_crown_status(ctx, emperor_entity_id);
        }

        let entity_id = self.entity_id;
        EmpireState::delete_shared(ctx, self.clone(), crate::inter_module::InterModuleDestination::AllOtherRegions);
        ctx.db.empire_lowercase_name_state().entity_id().delete(entity_id);
        ctx.db.empire_log_state().entity_id().delete(&entity_id);
        ctx.db.empire_emblem_state().entity_id().delete(entity_id);
    }

    pub fn merge_into(&self, ctx: &ReducerContext, new_empire_entity_id: u64) {
        // Delete foundries
        for foundry in ctx.db.empire_foundry_state().empire_entity_id().filter(self.entity_id) {
            delete_empire_building(ctx, 0, foundry.entity_id, false);
        }

        // Update all aligned nodes
        for mut empire_node in ctx.db.empire_node_state().empire_entity_id().filter(self.entity_id) {
            empire_node.empire_entity_id = new_empire_entity_id;
            EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        let mut new_empire_state = ctx.db.empire_state().entity_id().find(&new_empire_entity_id).unwrap();

        // Update all aligned settlements
        for mut settlement in ctx.db.empire_settlement_state().empire_entity_id().filter(self.entity_id) {
            settlement.empire_entity_id = new_empire_entity_id;
            EmpireSettlementState::update_shared(ctx, settlement, crate::inter_module::InterModuleDestination::AllOtherRegions);
            new_empire_state.num_claims += 1;
        }

        // Align all EmpireChunkStates
        for empire_chunk in ctx.db.empire_chunk_state().iter() {
            empire_chunk.convert_and_commit(ctx, self.entity_id, new_empire_entity_id);
        }

        // Remove crown from previous emperor
        if let Some(emperor) = ctx
            .db
            .empire_player_data_state()
            .empire_entity_id()
            .filter(self.entity_id)
            .filter(|data| data.rank == 0)
            .next()
        {
            EmpireState::remove_crown_status(ctx, emperor.entity_id);
        }

        // All aligned player ranks must be converted
        for mut rank in ctx.db.empire_player_data_state().empire_entity_id().filter(self.entity_id) {
            rank.empire_entity_id = new_empire_entity_id;
            rank.rank = 9; // retrograde to citizen for now
            EmpirePlayerDataState::update_shared(ctx, rank, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // Transfer treasury
        new_empire_state.shard_treasury += self.shard_treasury;
        new_empire_state.empire_currency_treasury += self.empire_currency_treasury;
        EmpireState::update_shared(ctx, new_empire_state, crate::inter_module::InterModuleDestination::AllOtherRegions);

        // all ongoing sieges for this empire will be transferred to the new empire
        for mut siege in ctx.db.empire_node_siege_state().empire_entity_id().filter(self.entity_id) {
            if siege.active {
                let mut attacking_siege = EmpireNodeSiegeState::get_attacking_siege(ctx, siege.building_entity_id).unwrap();
                let defending_siege = EmpireNodeSiegeState::get_defending_siege(ctx, siege.building_entity_id).unwrap();
                if attacking_siege.empire_entity_id == siege.empire_entity_id {
                    // this is the attacker, we will now simply cease this siege
                    siege.cancel_siege(ctx);
                } else {
                    // this is the defender
                    if defending_siege.empire_entity_id == new_empire_entity_id {
                        // it is attacked by the empire it merges into, let's conclude this siege as a successful attack that combines supplies
                        attacking_siege.energy += siege.energy;
                        siege.energy = 0;
                        let empire_node = ctx.db.empire_node_state().entity_id().find(&siege.building_entity_id).unwrap();
                        EmpireNodeSiegeState::end_siege_internal(ctx, attacking_siege, siege, empire_node);
                    } else {
                        // it is attacked by another empire, let's swap the defending siege allegiance
                        siege.empire_entity_id = new_empire_entity_id;
                        EmpireNodeSiegeState::update_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
                    }
                }
            } else {
                // Cancel all "marked for siege"
                EmpireNodeSiegeState::delete_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
            }
        }

        // delete all remaining stuff from the empire
        self.delete(ctx);
    }

    pub fn update_empire_upkeep(ctx: &ReducerContext, empire_entity_id: u64) {
        // NPC empire watchtowers use fixed upkeep values set at placement time
        if is_npc_empire(ctx, empire_entity_id) {
            return;
        }

        let empire = ctx.db.empire_state().entity_id().find(&empire_entity_id).unwrap();

        log::info!("Recalculating upkeep for every node");
        let chunk_index = SmallHexTile::from(empire.location).chunk_coordinates().chunk_index();
        let capital_coord = TerrainChunkState::chunk_coord_from_chunk_index(chunk_index);

        let nodes: Vec<EmpireNodeState> = ctx.db.empire_node_state().empire_entity_id().filter(empire_entity_id).collect();
        let active_node_count = nodes.clone().iter().filter(|n| n.active).count() as i32;

        let mut owned_chunks: Vec<u64> = ctx
            .db
            .empire_chunk_state()
            .empire_entity_id()
            .filter(empire_entity_id)
            .map(|c| c.chunk_index)
            .collect();

        let mut allied_settlements_chunk_indexes: Vec<u64> = ctx
            .db
            .empire_settlement_state()
            .empire_entity_id()
            .filter(empire_entity_id)
            .map(|settlement| settlement.chunk_index)
            .collect();
        allied_settlements_chunk_indexes.sort();
        allied_settlements_chunk_indexes.dedup();

        let mut areas = Vec::new(); // Chunks in this area, whether an allied claim can be found on those chunks

        while owned_chunks.len() > 0 {
            let chunk_index = owned_chunks[0];
            owned_chunks.remove(0);
            let area = Self::extract_connected_empire_chunks(ctx, chunk_index, &mut owned_chunks);
            let is_supplied_by_claim = allied_settlements_chunk_indexes
                .iter()
                .any(|chunk_index| area.contains(&chunk_index));
            areas.push((area, is_supplied_by_claim));
        }

        for mut node in nodes {
            let upkeep;
            let node_chunk_coord = TerrainChunkState::chunk_coord_from_chunk_index(node.chunk_index);
            if node.active {
                // A watchtower might have no territory if it has an adjacent opposing watchtower (after a succesful siege of one of two contiguous aligned watchtowers)
                let aligned_territory = areas.iter().filter(|a| a.0.contains(&node.chunk_index)).next();
                let is_supplied_by_claim = match aligned_territory {
                    Some(t) => t.1,
                    None => false,
                };
                let chunk_distance = i32::abs(node_chunk_coord.x - capital_coord.x) + i32::abs(node_chunk_coord.z - capital_coord.z);
                let node_count_cost = f32::log(active_node_count as f32, 2.0);
                let distance_cost = (chunk_distance as f32) * 0.2;
                let additional_constant = if is_supplied_by_claim { 1.0 } else { 16.0 };
                let empire_upkeep_multiplier = 0.1;
                upkeep = f32::ceil((node_count_cost + distance_cost + additional_constant) * empire_upkeep_multiplier) as i32;
                // log::info!("Upkeep update for node at {:?}", node_chunk_coord);
                // log::info!("Active Node Count... {} => {}", active_node_count, node_count_cost);
                // log::info!("Distance... {} => {}", chunk_distance, distance_cost);
                // log::info!("Additional Constant... {} => {}", is_supplied_by_claim, additional_constant);
                // log::info!("Total... {}", upkeep);
            } else {
                upkeep = 0;
                // log::info!("Upkeep update for INACTIVE node at {:?}", node_chunk_coord);
                // log::info!("Total... {}", upkeep);
            }

            if upkeep != node.upkeep {
                node.upkeep = upkeep;
                EmpireNodeState::update_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
            }
        }
    }

    fn extract_connected_empire_chunks(ctx: &ReducerContext, start: u64, owned_chunks: &mut Vec<u64>) -> HashSet<u64> {
        let world_info = ctx.db.world_region_state().id().find(0).unwrap();

        let min_x = world_info.region_min_chunk_x as i32;
        let min_z = world_info.region_min_chunk_z as i32;
        let max_x = min_x + world_info.region_width_chunks as i32 * world_info.region_count_sqrt as i32 - 1;
        let max_z = min_z + world_info.region_height_chunks as i32 * world_info.region_count_sqrt as i32 - 1;

        let mut open_set = Queue::new(); //vec![start];
        open_set.add(start).ok();

        let mut connected_chunks: HashSet<u64> = HashSet::new();
        connected_chunks.insert(start);

        //Flood-fill
        let neighbors = vec![(0, 1), (0, -1), (1, 0), (-1, 0)];
        while open_set.size() > 0 {
            let current = open_set.remove().ok().unwrap();
            let coord = TerrainChunkState::chunk_coord_from_chunk_index(current);
            for n in &neighbors {
                let neighbor_coord = ChunkCoordinates {
                    x: coord.x + n.0,
                    z: coord.z + n.1,
                    dimension: coord.dimension,
                };
                if neighbor_coord.x < min_x || neighbor_coord.x > max_x || neighbor_coord.z < min_z || neighbor_coord.z > max_z {
                    continue;
                }
                let neighbor_chunk_index = TerrainChunkState::chunk_index_from_coords(&neighbor_coord);
                if let Some(i) = owned_chunks.iter().position(|ci| *ci == neighbor_chunk_index) {
                    connected_chunks.insert(neighbor_chunk_index);
                    open_set.add(neighbor_chunk_index).ok();
                    owned_chunks.remove(i);
                }
            }
        }
        connected_chunks
    }

    pub fn remove_crown_status(ctx: &ReducerContext, player_entity_id: u64) {
        let region = game_state::player_region(ctx, player_entity_id).expect("Player region not found");
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::EmpireRemoveCrown(EmpireRemoveCrownMsg { player_entity_id }),
            crate::inter_module::InterModuleDestination::Region(region),
        );
    }

    pub fn update_crown_status(ctx: &ReducerContext, empire_entity_id: u64) {
        // NPC empire has no players, so no emperor to update crown for
        if is_npc_empire(ctx, empire_entity_id) {
            return;
        }

        let emperor = unwrap_or_return!(
            ctx.db
                .empire_player_data_state()
                .empire_entity_id()
                .filter(empire_entity_id)
                .filter(|d| d.rank == 0)
                .next(),
            "Emperor doesn't exist"
        );
        let region = game_state::player_region(ctx, emperor.entity_id).expect("Player region not found");
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::EmpireUpdateEmperorCrown(EmpireUpdateEmperorCrownMsg { empire_entity_id }),
            crate::inter_module::InterModuleDestination::Region(region),
        );
    }
}

impl EmpireSettlementState {
    pub fn leave_empire(mut self, ctx: &ReducerContext, claim_name: String) {
        let empire_entity_id = self.empire_entity_id;
        self.empire_entity_id = 0;
        EmpireSettlementState::update_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);

        EmpireState::update_empire_upkeep(ctx, empire_entity_id);

        let mut empire = ctx.db.empire_state().entity_id().find(&empire_entity_id).unwrap();
        empire.num_claims -= 1;
        EmpireState::update_shared(ctx, empire, crate::inter_module::InterModuleDestination::AllOtherRegions);

        // Claim Left Notification (12)
        EmpireNotificationState::new(ctx, EmpireNotificationType::ClaimLeft, empire_entity_id, vec![claim_name]);
    }

    pub fn update_donations(ctx: &ReducerContext, claim_entity_id: u64, ignored_player_eid: u64) -> Result<(), String> {
        if let Some(mut settlement) = Self::from_claim(ctx, claim_entity_id) {
            let sum: u32 = ctx
                .db
                .claim_member_state()
                .claim_entity_id()
                .filter(&claim_entity_id)
                .map(|m| m.player_entity_id)
                .map(|eid| match ctx.db.empire_player_data_state().entity_id().find(&eid) {
                    Some(rank) => {
                        if rank.empire_entity_id == settlement.empire_entity_id {
                            if rank.entity_id == ignored_player_eid {
                                0
                            } else {
                                rank.donated_shards + rank.donated_empire_currency
                            }
                        } else {
                            0
                        }
                    }
                    None => 0,
                })
                .sum();
            settlement.members_donations = sum;
            EmpireSettlementState::update_shared(ctx, settlement, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
        Ok(())
    }

    // Update all empire nodes the player is part of
    pub fn update_donations_from_player(
        ctx: &ReducerContext,
        player_entity_id: u64,
        ignore_player_contributions: bool,
    ) -> Result<(), String> {
        let empire_entity_id = unwrap_or_err!(
            ctx.db.empire_player_data_state().entity_id().find(&player_entity_id),
            "Player is not part of an empire"
        )
        .empire_entity_id;

        let player_claims = ctx
            .db
            .empire_node_state()
            .empire_entity_id()
            .filter(empire_entity_id)
            .map(|node| node.entity_id)
            .filter_map(|eid| {
                if ctx
                    .db
                    .claim_member_state()
                    .player_claim()
                    .filter((player_entity_id, eid))
                    .next()
                    .is_some()
                {
                    Some(eid)
                } else {
                    None
                }
            });

        for claim_entity_id in player_claims {
            Self::update_donations(ctx, claim_entity_id, if ignore_player_contributions { player_entity_id } else { 0 })?;
        }
        Ok(())
    }
}

impl EmpireChunkState {
    pub fn stamp_and_commit(mut self, ctx: &ReducerContext, empire_entity_id: u64) {
        self.empire_entity_id = empire_entity_id;
        EmpireChunkState::update_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    pub fn unstamp_and_commit(mut self, ctx: &ReducerContext, empire_entity_id: u64) {
        if self.empire_entity_id == empire_entity_id {
            self.empire_entity_id = 0;
            self.apply_transaction(ctx);
        }
    }

    pub fn unstamp_all_and_commit(mut self, ctx: &ReducerContext, empire_entity_id: u64) {
        if self.empire_entity_id == empire_entity_id {
            self.empire_entity_id = 0;
        }
        self.apply_transaction(ctx);
    }

    pub fn convert_and_commit(mut self, ctx: &ReducerContext, previous_empire_entity_id: u64, new_empire_entity_id: u64) {
        if self.empire_entity_id == previous_empire_entity_id {
            self.empire_entity_id = new_empire_entity_id;
        }
        EmpireChunkState::update_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    fn apply_transaction(self, ctx: &ReducerContext) {
        if self.empire_entity_id == 0 && self.watchtower_entity_id == 0 {
            // Only delete chunks that were NOT pre-assigned to a watchtower.
            // Watchtower territory chunks are permanent and should remain with empty ownership.
            EmpireChunkState::delete_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
        } else {
            EmpireChunkState::update_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
    }
}

impl EmpirePlayerDataState {
    pub fn new(ctx: &ReducerContext, actor_id: u64, empire_entity_id: u64, rank: u8) -> Result<(), String> {
        EmpirePlayerDataState::insert_shared(
            ctx,
            EmpirePlayerDataState {
                entity_id: actor_id,
                empire_entity_id,
                rank,
                donated_shards: 0,
                noble: None,
                donated_empire_currency: 0,
            },
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );

        let user = ctx.db.user_state().entity_id().find(actor_id).unwrap();
        let region = unwrap_or_err!(ctx.db.user_region_state().identity().find(user.identity), "Player region not found").region_id;
        send_inter_module_message(
            ctx,
            crate::messages::inter_module::MessageContentsV5::OnPlayerJoinedEmpire(OnPlayerJoinedEmpireMsg {
                player_entity_id: actor_id,
                empire_entity_id,
            }),
            crate::inter_module::InterModuleDestination::Region(region),
        );

        Ok(())
    }

    pub fn is_emperor(ctx: &ReducerContext, player_entity_id: u64, empire_entity_id: u64) -> bool {
        if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(&player_entity_id) {
            return rank.rank == 0 && rank.empire_entity_id == empire_entity_id;
        }
        false
    }

    pub fn is_part_of_empire(ctx: &ReducerContext, player_entity_id: u64, empire_entity_id: u64) -> bool {
        if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(&player_entity_id) {
            return rank.empire_entity_id == empire_entity_id;
        }
        false
    }

    pub fn get_player_empire_id(ctx: &ReducerContext, player_entity_id: u64) -> Option<u64> {
        if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(&player_entity_id) {
            return Some(rank.empire_entity_id);
        }
        None
    }
}

impl EmpireNodeSiegeState {
    pub fn get_attacking_siege(ctx: &ReducerContext, building_entity_id: u64) -> Option<EmpireNodeSiegeState> {
        if let Some(empire_node) = ctx.db.empire_node_state().entity_id().find(&building_entity_id) {
            let sieges_on_node = ctx.db.empire_node_siege_state().building_entity_id().filter(building_entity_id);
            return sieges_on_node
                .filter(|s| s.active && s.empire_entity_id != empire_node.empire_entity_id)
                .next();
        }
        None
    }

    pub fn get_defending_siege(ctx: &ReducerContext, building_entity_id: u64) -> Option<EmpireNodeSiegeState> {
        if let Some(empire_node) = ctx.db.empire_node_state().entity_id().find(&building_entity_id) {
            let sieges_on_node = ctx.db.empire_node_siege_state().building_entity_id().filter(building_entity_id);
            return sieges_on_node
                .filter(|s| s.active && s.empire_entity_id == empire_node.empire_entity_id)
                .next();
        }
        None
    }

    pub fn end_siege(ctx: &ReducerContext, building_entity_id: u64, sieging_empire_entity_id: u64) {
        let siege = Self::get(ctx, building_entity_id, sieging_empire_entity_id).unwrap();
        let empire_node = ctx.db.empire_node_state().entity_id().find(&building_entity_id).unwrap();
        let defense = Self::get(ctx, building_entity_id, empire_node.empire_entity_id).unwrap();
        Self::end_siege_internal(ctx, siege, defense, empire_node);
    }

    fn end_siege_internal(
        ctx: &ReducerContext,
        siege: EmpireNodeSiegeState,
        defense: EmpireNodeSiegeState,
        mut empire_node: EmpireNodeState,
    ) {
        let coord = empire_node.location.into();

        // Destroy any siege engine related to this siege
        if let Some(siege_engine) = ctx
            .db
            .empire_siege_engine_state()
            .building_entity_id()
            .find(&defense.building_entity_id)
        {
            ctx.db.empire_siege_engine_state().entity_id().delete(&siege_engine.entity_id);

            send_inter_module_message(
                ctx,
                crate::messages::inter_module::MessageContentsV5::RegionDestroySiegeEngine(RegionDestroySiegeEngineMsg {
                    deployable_entity_id: siege_engine.entity_id,
                }),
                crate::inter_module::InterModuleDestination::Region(game_state::region_index_from_entity_id(defense.building_entity_id)),
            );
        }

        let node_entity_id = empire_node.entity_id;

        if siege.energy > 0 {
            // successful siege, switch tower allegience
            empire_node.convert(ctx, siege.energy, siege.empire_entity_id);
            // Successful Siege (5)
            EmpireNotificationState::new_with_nickname_and_coord(
                ctx,
                EmpireNotificationType::SuccessfulSiege,
                siege.empire_entity_id,
                node_entity_id,
                coord,
            );
            // Failed Defense (8)
            EmpireNotificationState::new_with_nickname_and_coord(
                ctx,
                EmpireNotificationType::FailedDefense,
                defense.empire_entity_id,
                node_entity_id,
                coord,
            );

            EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);

            // Nodes changed, update both empires upkeeps
            EmpireState::update_empire_upkeep(ctx, defense.empire_entity_id);
            EmpireState::update_empire_upkeep(ctx, siege.empire_entity_id);
        } else {
            // successful defense, keep extra supplies
            empire_node.add_energy(ctx, defense.energy, None);
            EmpireNodeState::update_shared(ctx, empire_node, crate::inter_module::InterModuleDestination::AllOtherRegions);

            // Failed Siege (7)
            EmpireNotificationState::new_with_nickname_and_coord(
                ctx,
                EmpireNotificationType::FailedSiege,
                siege.empire_entity_id,
                node_entity_id,
                coord,
            );
            // Successful Defense (6)
            EmpireNotificationState::new_with_nickname_and_coord(
                ctx,
                EmpireNotificationType::SuccessfulDefense,
                defense.empire_entity_id,
                node_entity_id,
                coord,
            );
        }
        // Delete both sieges
        EmpireNodeSiegeState::delete_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
        EmpireNodeSiegeState::delete_shared(ctx, defense, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    pub fn cancel_siege(self, ctx: &ReducerContext) {
        if self.active {
            // Need to cancel the ongoing siege for both parties. The cancelled siege will drop to 0 supplies and lose.
            let mut empire_node = ctx.db.empire_node_state().entity_id().find(&self.building_entity_id).unwrap();
            let mut siege;
            let mut defense;
            if self.empire_entity_id == empire_node.empire_entity_id {
                // This siege shares the tower's empire, therefore it's the defending side
                empire_node.energy = 0;
                defense = self.clone();
                defense.energy = 0;
                siege = ctx
                    .db
                    .empire_node_siege_state()
                    .building_entity_id()
                    .filter(empire_node.entity_id)
                    .filter(|s| s.active && s.empire_entity_id != self.empire_entity_id)
                    .next()
                    .unwrap();
            } else {
                // This siege has a different empire than the tower's, therefore it's the attacking side
                defense = EmpireNodeSiegeState::get(ctx, empire_node.entity_id, empire_node.empire_entity_id).unwrap();
                siege = self.clone();
                siege.energy = 0;
            }
            Self::end_siege_internal(ctx, siege, defense, empire_node);
        } else {
            // Marked for siege but no active siege, just remove the siege state.
            EmpireNodeSiegeState::delete_shared(ctx, self, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }
    }

    pub fn start_siege(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
        supplies: i32,
        coord: SmallHexTile,
        dry_run: bool,
    ) -> Result<(), String> {
        Self::validate_action(ctx, actor_id, building_entity_id)?;

        let rank = ctx.db.empire_player_data_state().entity_id().find(&actor_id).unwrap();
        let mut defending_node = ctx.db.empire_node_state().entity_id().find(&building_entity_id).unwrap();

        let defending_empire = defending_node.empire_entity_id;

        if defending_empire == rank.empire_entity_id {
            return Err("You cannot siege one of your own watchtowers".into());
        }

        // No active siege, let's confirm that this watchtower is eligible to start a siege.
        if let Some(mut siege) = EmpireNodeSiegeState::get(ctx, building_entity_id, rank.empire_entity_id) {
            if !dry_run {
                // Instant resolution if node has 0 supplies
                if defending_node.energy == 0 {
                    if !dry_run {
                        let defending_empire_entity_id = defending_node.empire_entity_id;
                        let building_entity_id = defending_node.entity_id;
                        // switch tower allegience
                        defending_node.convert(ctx, supplies, rank.empire_entity_id);
                        EmpireNodeState::update_shared(ctx, defending_node, crate::inter_module::InterModuleDestination::AllOtherRegions);
                        // DAB Note: Do we want different notifications for instant sieges?
                        // Successful Siege (5)
                        EmpireNotificationState::new_with_nickname_and_coord(
                            ctx,
                            EmpireNotificationType::SuccessfulSiege,
                            rank.empire_entity_id,
                            building_entity_id,
                            coord,
                        );
                        // Failed Defense (8)
                        EmpireNotificationState::new_with_nickname_and_coord(
                            ctx,
                            EmpireNotificationType::FailedDefense,
                            defending_empire,
                            building_entity_id,
                            coord,
                        );

                        if let Some(previous_siege) = EmpireNodeSiegeState::get(ctx, building_entity_id, rank.empire_entity_id) {
                            previous_siege.cancel_siege(ctx);
                        }

                        //Remove all siege marks
                        for s in ctx.db.empire_node_siege_state().building_entity_id().filter(building_entity_id) {
                            EmpireNodeSiegeState::delete_shared(ctx, s, crate::inter_module::InterModuleDestination::AllOtherRegions);
                        }

                        // Nodes changed, update both empires upkeeps
                        EmpireState::update_empire_upkeep(ctx, defending_empire_entity_id);
                        EmpireState::update_empire_upkeep(ctx, rank.empire_entity_id);
                    }
                    return Ok(());
                }

                // Activate attacker siege
                let attacker_siege_entity_id = siege.entity_id;
                siege.energy = supplies;
                siege.active = true;
                siege.start_timestamp = Some(ctx.timestamp);
                EmpireNodeSiegeState::update_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);

                //Remove all other siege marks
                for s in ctx.db.empire_node_siege_state().building_entity_id().filter(building_entity_id) {
                    if s.entity_id != attacker_siege_entity_id {
                        EmpireNodeSiegeState::delete_shared(ctx, s, crate::inter_module::InterModuleDestination::AllOtherRegions);
                    }
                }

                // Add defender siege
                let entity_id = game_state::create_entity(ctx);
                let siege_node = EmpireNodeSiegeState {
                    entity_id,
                    building_entity_id: building_entity_id,
                    empire_entity_id: defending_empire,
                    energy: 0,
                    active: true,
                    start_timestamp: Some(ctx.timestamp),
                };

                EmpireNodeSiegeState::insert_shared(ctx, siege_node, crate::inter_module::InterModuleDestination::AllOtherRegions);

                // Started Siege (3)
                EmpireNotificationState::new_with_nickname_and_coord(
                    ctx,
                    EmpireNotificationType::StartedSiege,
                    rank.empire_entity_id,
                    building_entity_id,
                    coord,
                );
                // Started Defense (4)
                EmpireNotificationState::new_with_nickname_and_coord(
                    ctx,
                    EmpireNotificationType::StartedDefense,
                    defending_empire,
                    building_entity_id,
                    coord,
                );
            }
        } else {
            return Err("This node is not marked for siege".into());
        }
        Ok(())
    }
}

impl EmpireNotificationState {
    pub fn coord_to_string(coord: SmallHexTile) -> String {
        let large_tile = coord.parent_large_tile().to_offset_coordinates();
        format!("N:{{0}}, E:{{1}}|~{}|~{}", large_tile.z, large_tile.x)
    }

    pub fn new_with_nickname_and_coord(
        ctx: &ReducerContext,
        notification_type: EmpireNotificationType,
        empire_entity_id: u64,
        building_entity_id: u64,
        coord: SmallHexTile,
    ) {
        let mut nickname = ctx
            .db
            .building_nickname_state()
            .entity_id()
            .find(&building_entity_id)
            .unwrap()
            .nickname;
        if nickname.is_empty() {
            nickname = "Watchtower".to_string()
        };
        let replacement_text = vec![nickname, Self::coord_to_string(coord)];
        Self::new(ctx, notification_type, empire_entity_id, replacement_text)
    }

    pub fn new_with_coord(ctx: &ReducerContext, notification_type: EmpireNotificationType, empire_entity_id: u64, coord: SmallHexTile) {
        Self::new(ctx, notification_type, empire_entity_id, vec![Self::coord_to_string(coord)])
    }

    pub fn new(ctx: &ReducerContext, notification_type: EmpireNotificationType, empire_entity_id: u64, text: Vec<String>) {
        let entity_id = game_state::create_entity(ctx);
        for mut player in ctx.db.empire_player_log_state().empire_entity_id().filter(empire_entity_id) {
            if ctx.db.signed_in_player_state().entity_id().find(&player.entity_id).is_some() {
                player.last_viewed = entity_id;
                ctx.db.empire_player_log_state().entity_id().update(player);
            }
        }

        let mut empire_log = ctx.db.empire_log_state().entity_id().find(&empire_entity_id).unwrap();
        empire_log.last_posted = entity_id;
        ctx.db.empire_log_state().entity_id().update(empire_log);

        ctx.db
            .empire_notification_state()
            .try_insert(EmpireNotificationState {
                entity_id,
                empire_entity_id,
                notification_type,
                text_replacement: text,
                timestamp: unix(ctx.timestamp),
            })
            .unwrap();
    }
}
