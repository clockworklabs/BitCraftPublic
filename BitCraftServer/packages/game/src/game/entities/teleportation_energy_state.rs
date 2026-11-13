use spacetimedb::{log, ReducerContext};

use crate::{
    game::{coordinates::region_coordinates::RegionCoordinates, dimensions, game_state::game_state_filters},
    messages::{
        components::{character_stats_state, teleportation_energy_state, TeleportationEnergyState},
        generic::world_region_state,
        static_data::{parameters_desc, CharacterStatType},
    },
    params,
    utils::from_ctx::FromCtx,
};

use super::{building_state::InventoryState, enemy_state::FloatHexTile};

impl TeleportationEnergyState {
    pub fn get(ctx: &ReducerContext, actor_id: u64) -> Self {
        ctx.db.teleportation_energy_state().entity_id().find(actor_id).unwrap()
    }

    pub fn update(self, ctx: &ReducerContext) {
        ctx.db.teleportation_energy_state().entity_id().update(self);
    }

    pub fn add_energy_regen(&mut self, ctx: &ReducerContext) -> bool {
        if let Some(character_stats_state) = ctx.db.character_stats_state().entity_id().find(self.entity_id) {
            let max_energy = character_stats_state.get(CharacterStatType::MaxTeleportationEnergy);
            let energy_regen = character_stats_state.get(CharacterStatType::TeleportationEnergyRegenRate);
            if self.energy >= max_energy && energy_regen >= 0.0 {
                return false;
            }
            if self.energy <= 0.0 && energy_regen <= 0.0 {
                return false;
            }
            let energy = (self.energy + energy_regen).clamp(0.0, max_energy);
            self.energy = energy;
            return true;
        }
        false
    }

    pub fn add_energy(&mut self, ctx: &ReducerContext, energy_gain: f32) -> bool {
        if let Some(character_stats_state) = ctx.db.character_stats_state().entity_id().find(self.entity_id) {
            let max_energy = character_stats_state.get(CharacterStatType::MaxTeleportationEnergy);
            if self.energy >= max_energy && energy_gain >= 0.0 {
                return false;
            }
            if self.energy <= 0.0 && energy_gain <= 0.0 {
                return false;
            }
            let energy = (self.energy + energy_gain).clamp(0.0, max_energy);
            self.energy = energy;
            return true;
        }
        false
    }

    pub fn expend_energy(&mut self, energy_cost: f32, allow_negative: bool) -> bool {
        if !allow_negative && self.energy < energy_cost {
            return false;
        }
        self.energy -= energy_cost;
        true
    }

    pub fn teleport_home_cost(&self, ctx: &ReducerContext, _from_death: bool) -> f32 {
        // Teleporting home after death currently costs the same as a regular teleport home.
        let base_cost = ctx.db.parameters_desc().version().find(0).unwrap().teleportation_home_energy_cost as i32;
        base_cost as f32
    }

    pub fn teleport_cost(&self, ctx: &ReducerContext, destination: FloatHexTile) -> Option<f32> {
        let params = params!(ctx);

        let base_cost = params.teleportation_base_energy_cost;
        let location = game_state_filters::coordinates_float(ctx, self.entity_id).parent_large_tile();
        if location.dimension != dimensions::OVERWORLD || destination.dimension != dimensions::OVERWORLD {
            log::error!("Teleporting to a specific location can only be done in the overworld");
            return None;
        }

        let distance = destination.parent_large_tile().distance_to(location) as f32;
        let distance_cost = distance * params.teleportation_cost_per_large_tile;

        let region = ctx.db.world_region_state().id().find(0).unwrap();
        let destination_region = RegionCoordinates::from_ctx(ctx, destination).to_region_index(region.region_count_sqrt);
        if destination_region == region.region_index {
            return Some((base_cost + distance_cost).ceil());
        }

        let inventory = InventoryState::get_player_inventory(ctx, self.entity_id).unwrap();
        let filled_pockets = inventory
            .pockets
            .iter()
            .filter(|p| p.contents.is_some() && p.contents.unwrap().quantity > 0)
            .count();
        let total_pockets = inventory.pockets.len();

        let inventory_multiplier =
            1.0 + (filled_pockets as f32 / total_pockets as f32) * (params.teleportation_full_inventory_multiplier - 1.0);
        Some(((base_cost + distance_cost) * inventory_multiplier).ceil())
    }
}
