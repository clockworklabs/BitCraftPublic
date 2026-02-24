use std::collections::HashSet;

use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{
        discovery::Discovery,
        game_state::{self, game_state_filters},
        handlers::{
            player_vault::deployable_move_off_bounds::deployable_move_off_bounds,
            server::server_teleport_player::{teleport_player_timer, TeleportPlayerTimer},
        },
        reducer_helpers::interior_helpers,
    },
    messages::{
        action_request::ServerTeleportReason,
        components::*,
        game_util::ItemType,
        generic::world_region_state,
        static_data::{building_desc, player_housing_desc, BuildingCategory, BuildingFunction},
    },
    unwrap_or_err,
};

use super::duel_state::{ChunkCoordinates, OffsetCoordinatesSmall};

impl PlayerHousingState {
    pub fn from_dimension(ctx: &ReducerContext, dimension: u32) -> Option<Self> {
        if let Some(dimension_descriptor) = ctx.db.dimension_description_state().dimension_id().find(dimension) {
            return ctx
                .db
                .player_housing_state()
                .network_entity_id()
                .find(dimension_descriptor.dimension_network_entity_id);
        }
        None
    }

    pub fn set_permissions(
        &self,
        ctx: &ReducerContext,
        permission_group: PermissionGroup,
        permission: Permission,
        allowed_entity_id: u64,
    ) -> Result<(), String> {
        // Tag interiors with Owner PermissionState
        let ordained_entities = self.get_permission_entities(ctx);

        for ordained_entity_id in ordained_entities {
            let permission_state = PermissionState::new(ctx, ordained_entity_id, allowed_entity_id, permission_group, permission)?;
            ctx.db.permission_state().insert(permission_state);
        }
        Ok(())
    }

    pub fn copy_permissions(&self, ctx: &ReducerContext) -> Result<(), String> {
        // The permissions are found on the player housing component itself, so we don't need the previous network
        let existing_permissions: Vec<PermissionState> = ctx.db.permission_state().ordained_entity_id().filter(self.entity_id).collect();

        // Don't retrieve the permissions already set on the player housing
        let ordained_entities = Self::get_permission_entities_from_network_id(ctx, self.network_entity_id);

        // Apply player housing's permissions to all dimensions in its network
        for ordained_entity_id in ordained_entities {
            for permission in existing_permissions.iter() {
                let mut permission_state = permission.clone();
                permission_state.entity_id = game_state::create_entity(ctx);
                permission_state.ordained_entity_id = ordained_entity_id;
                ctx.db.permission_state().insert(permission_state);
            }
        }
        Ok(())
    }

    pub fn get_permission_entities_from_network_id(ctx: &ReducerContext, network_entity_id: u64) -> Vec<u64> {
        // entities to update are all dimensions
        ctx.db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(network_entity_id)
            .map(|d| d.entity_id)
            .collect()
    }

    pub fn get_permission_entities(&self, ctx: &ReducerContext) -> Vec<u64> {
        let mut entities = Self::get_permission_entities_from_network_id(ctx, self.network_entity_id);
        // and player housing itself (for ease of editing)
        entities.push(self.entity_id);
        entities
    }

    pub fn is_empty(&self, ctx: &ReducerContext) -> bool {
        if let Some(moving_cost) = ctx.db.player_housing_moving_cost_state().entity_id().find(self.entity_id) {
            return moving_cost.moving_time_cost_minutes == 0;
        }
        true
    }

    pub fn is_dimension_network_empty(ctx: &ReducerContext, network_entity_id: u64) -> bool {
        for dimension_description in ctx
            .db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(network_entity_id)
        {
            for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
                if ctx.db.portal_state().entity_id().find(location.entity_id).is_none()
                    && ctx.db.footprint_tile_state().entity_id().find(location.entity_id).is_none()
                    //Item piles are deleted
                    && ctx.db.dropped_inventory_state().entity_id().find(location.entity_id).is_none()
                {
                    // found a location with no portal associated to it, therefore the housing is not empty
                    // EDIT: There is a bug where orphaned locations can remain behind. For now, we will prevent the move if we find:
                    // - a building, a resource, a paved_tile_state, a pillar_shaping
                    if ctx.db.building_state().entity_id().find(location.entity_id).is_some()
                        || ctx.db.resource_state().entity_id().find(location.entity_id).is_some()
                        || ctx.db.paved_tile_state().entity_id().find(location.entity_id).is_some()
                        || ctx.db.pillar_shaping_state().entity_id().find(location.entity_id).is_some()
                    {
                        return false;
                    } else {
                        log::error!(
                            "Found an orphaned location moving dimension {} from network {}",
                            dimension_description.dimension_id,
                            network_entity_id
                        );
                    }
                }
            }
            let chunk = ChunkCoordinates {
                x: 0,
                z: 0,
                dimension: dimension_description.dimension_id,
            };
            for mobile in ctx.db.mobile_entity_state().chunk_index().filter(chunk.chunk_index()) {
                if ctx.db.player_state().entity_id().find(mobile.entity_id).is_some()
                    || ctx.db.deployable_state().entity_id().find(mobile.entity_id).is_some()
                {
                    // ignore players and deployables as they will be warped outside
                } else {
                    // found a non-player mobile entity in the house, therefore it's not empty
                    return false;
                }
            }
        }
        true
    }

    pub fn get_all_player_housing_inventories(ctx: &ReducerContext, network_entity_id: u64) -> Vec<InventoryState> {
        let mut inventories = Vec::new();
        let mut inventory_ids: HashSet<u64> = HashSet::new();

        for dimension_description in ctx
            .db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(network_entity_id)
        {
            for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
                if let Some(inventory) = ctx.db.inventory_state().owner_entity_id().filter(location.entity_id).next() {
                    if !inventory_ids.contains(&inventory.entity_id) {
                        inventory_ids.insert(inventory.entity_id);
                        inventories.push(inventory);
                    }
                }
            }
            let chunk = ChunkCoordinates {
                x: 0,
                z: 0,
                dimension: dimension_description.dimension_id,
            };
            for mobile in ctx.db.mobile_entity_state().chunk_index().filter(chunk.chunk_index()) {
                if let Some(inventory) = ctx.db.inventory_state().owner_entity_id().filter(mobile.entity_id).next() {
                    if !inventory_ids.contains(&inventory.entity_id) {
                        inventory_ids.insert(inventory.entity_id);
                        inventories.push(inventory);
                    }
                }
            }
        }
        inventories
    }

    pub fn get_player_housing_building_inventories(ctx: &ReducerContext, network_entity_id: u64) -> Vec<InventoryState> {
        let mut inventories = Vec::new();
        let mut inventory_ids: HashSet<u64> = HashSet::new();

        for dimension_description in ctx
            .db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(network_entity_id)
        {
            for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
                if ctx.db.building_state().entity_id().find(location.entity_id).is_some() {
                    if let Some(inventory) = ctx.db.inventory_state().owner_entity_id().filter(location.entity_id).next() {
                        if !inventory_ids.contains(&inventory.entity_id) {
                            inventory_ids.insert(inventory.entity_id);
                            inventories.push(inventory);
                        }
                    }
                }
            }
        }
        inventories
    }

    pub fn get_housing_move_minutes_cost(ctx: &ReducerContext, network_entity_id: u64) -> i32 {
        const MAX_COST: i32 = 28800; //20 days
        const BASE_COST: i32 = 720; //12 hours
        const ITEM_COST: i32 = 30;
        const CARGO_COST: i32 = 10;

        let inventories = Self::get_player_housing_building_inventories(ctx, network_entity_id);
        let mut minutes = 0;
        for inv in inventories {
            minutes += inv
                .pockets
                .iter()
                .filter_map(|p| {
                    if let Some(content) = p.contents {
                        if content.item_type == ItemType::Item {
                            Some(ITEM_COST)
                        } else {
                            Some(CARGO_COST * content.quantity)
                        }
                    } else {
                        None
                    }
                })
                .sum::<i32>();
        }

        if minutes != 0 {
            minutes += BASE_COST;
        }

        minutes.min(MAX_COST)
    }

    pub fn get_and_validate_player_housing(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
        requires_entrance_building: bool,
        housing_owner_entity_id: u64,
    ) -> Result<(PlayerHousingState, BuildingState), String> {
        let building = Self::get_and_validate_entrance_building(ctx, actor_id, building_entity_id)?;

        let player_housing_entity_id = housing_owner_entity_id;
        if let Some(player_housing) = ctx.db.player_housing_state().entity_id().find(player_housing_entity_id) {
            if requires_entrance_building && player_housing.entrance_building_entity_id != building_entity_id {
                return Err("You cannot do this from this building".into());
            }
            return Ok((player_housing, building));
        }

        if player_housing_entity_id == actor_id {
            return Err("You do not own a personal house".into());
        }
        Err("This player does not own a personal house".into())
    }

    pub fn get_and_validate_entrance_building(
        ctx: &ReducerContext,
        actor_id: u64,
        building_entity_id: u64,
    ) -> Result<BuildingState, String> {
        let player_coord = game_state_filters::coordinates_any(ctx, actor_id);

        let building = unwrap_or_err!(
            ctx.db.building_state().entity_id().find(building_entity_id),
            "Could not find housing portal"
        );
        if building.distance_to(ctx, &player_coord) > 2 {
            return Err("Too far".into());
        }

        let building_desc = ctx.db.building_desc().id().find(building.building_description_id).unwrap();
        if !building_desc.has_category(ctx, BuildingCategory::PlayerHousing) {
            return Err("This building cannot be used for player housing".into());
        }

        Ok(building)
    }

    pub fn expel_players_and_entities(&self, ctx: &ReducerContext, reason: ServerTeleportReason) -> Vec<u64> {
        // Expels players and deployables. Deletes item piles
        let portal_state = ctx.db.portal_state().entity_id().find(self.exit_portal_entity_id).unwrap();

        // Teleport players outside
        let teleport_oc_small = OffsetCoordinatesSmall {
            x: portal_state.destination_x,
            z: portal_state.destination_z,
            dimension: portal_state.destination_dimension,
        };
        let teleport_oc_float = teleport_oc_small.into();

        let mut expelled_players = Vec::new();

        for dimension_description in ctx
            .db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(self.network_entity_id)
        {
            let chunk = ChunkCoordinates {
                x: 0,
                z: 0,
                dimension: dimension_description.dimension_id,
            };
            for mobile in ctx.db.mobile_entity_state().chunk_index().filter(chunk.chunk_index()) {
                if ctx.db.player_state().entity_id().find(mobile.entity_id).is_some() {
                    expelled_players.push(mobile.entity_id);
                    ctx.db.teleport_player_timer().insert(TeleportPlayerTimer {
                        scheduled_id: 0,
                        scheduled_at: ctx.timestamp.into(),
                        player_entity_id: mobile.entity_id,
                        location: teleport_oc_float,
                        reason,
                    });
                } else if ctx.db.deployable_state().entity_id().find(mobile.entity_id).is_some() {
                    ctx.db.mounting_state().deployable_entity_id().delete(mobile.entity_id);
                    let mes = MobileEntityState::for_location(mobile.entity_id, teleport_oc_float, ctx.timestamp);
                    ctx.db.mobile_entity_state().entity_id().update(mes);
                    let _ = deployable_move_off_bounds(ctx, mobile.entity_id);
                }
            }

            for loc in ctx.db.location_state().chunk_index().filter(chunk.chunk_index()) {
                if let Some(dropped_inventory) = ctx.db.dropped_inventory_state().entity_id().find(loc.entity_id) {
                    dropped_inventory.delete(ctx);
                }
            }
        }
        expelled_players
    }

    pub fn update_is_empty_flag(ctx: &ReducerContext, dimension: u32) {
        // Verify empty state of player Housing if a player signs off in a player house or leaves it
        if let Some(dimension_description) = ctx.db.dimension_description_state().dimension_id().find(dimension) {
            if let Some(mut player_housing) = ctx
                .db
                .player_housing_state()
                .network_entity_id()
                .find(dimension_description.dimension_network_entity_id)
            {
                let was_empty = player_housing.is_empty;
                player_housing.update_is_empty_flag_self(ctx);
                // Player is leaving a player housing, update the is_empty flag
                if was_empty != player_housing.is_empty {
                    PlayerHousingState::update_shared(
                        ctx,
                        player_housing,
                        crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
                    );
                }
            }
        }
    }

    pub fn update_is_empty_flag_self(&mut self, ctx: &ReducerContext) {
        // Verify empty state of player Housing if a player signs off in a player house or leaves it
        let player_housing_entity_id = self.entity_id;
        // Player is leaving a player housing, update the is_empty flag
        let minute_cost = Self::get_housing_move_minutes_cost(ctx, self.network_entity_id);
        let previous_cost = ctx
            .db
            .player_housing_moving_cost_state()
            .entity_id()
            .find(player_housing_entity_id)
            .unwrap()
            .moving_time_cost_minutes;
        self.is_empty = minute_cost == 0;
        if minute_cost != previous_cost {
            ctx.db
                .player_housing_moving_cost_state()
                .entity_id()
                .update(PlayerHousingMovingCostState {
                    entity_id: player_housing_entity_id,
                    moving_time_cost_minutes: minute_cost,
                });
        }
    }

    pub fn create_housing(
        ctx: &ReducerContext,
        actor_id: u64,
        rank: i32,
        template_building_id: i32,
        entrance_building: &BuildingState,
        outside_dimension: u32,
    ) -> Result<(), String> {
        let building_entity_id = entrance_building.entity_id;

        // see if there is enough room for a new tenant
        let building_desc = unwrap_or_err!(
            ctx.db.building_desc().id().find(entrance_building.building_description_id),
            "Invalid building"
        );

        let housing_slots = BuildingFunction::max_housing_slots(&building_desc) as usize;
        if housing_slots == 0 {
            return Err("This building can't house players".into());
        }
        if housing_slots > 0 {
            let current_houses = ctx
                .db
                .player_housing_state()
                .entrance_building_entity_id()
                .filter(building_entity_id)
                .count();
            if current_houses >= housing_slots {
                return Err("This building cannot house anymore players".into());
            }
        }

        // Create new custom interior
        let network_entity_id = interior_helpers::create_player_interior(ctx, template_building_id, building_entity_id)?;

        let region = ctx.db.world_region_state().iter().next().unwrap();
        // New player housing component on player
        let player_housing = PlayerHousingState {
            entity_id: actor_id,
            entrance_building_entity_id: building_entity_id,
            rank,
            network_entity_id,
            exit_portal_entity_id: Self::find_portal_to_outside(ctx, network_entity_id, outside_dimension),
            locked_until: ctx.timestamp,
            is_empty: true,
            region_index: region.region_index,
        };

        // Default permissions
        player_housing.set_permissions(ctx, PermissionGroup::Everyone, Permission::Visitor, 0)?;
        player_housing.set_permissions(ctx, PermissionGroup::Player, Permission::Owner, actor_id)?;

        PlayerHousingState::insert_shared(
            ctx,
            player_housing,
            crate::inter_module::InterModuleDestination::GlobalAndAllOtherRegions,
        );

        // This is only used for same-region transfers, therefore this doesn't need to be shared with global module
        let player_housing_moving_cost_state = PlayerHousingMovingCostState {
            entity_id: actor_id,
            moving_time_cost_minutes: 0,
        };
        ctx.db.player_housing_moving_cost_state().insert(player_housing_moving_cost_state);

        ctx.db.player_housing_customization_state().insert(PlayerHousingCustomizationState {
            entity_id: actor_id,
            wall_collectible_id: 0,
            floor_collectible_id: 0,
        });
        Ok(())
    }

    pub fn find_portal_to_outside(ctx: &ReducerContext, network_entity_id: u64, outside_dimension: u32) -> u64 {
        // Find which portal leads to the exterior
        for dimension_description in ctx
            .db
            .dimension_description_state()
            .dimension_network_entity_id()
            .filter(network_entity_id)
        {
            for location in ctx.db.location_state().dimension_filter(dimension_description.dimension_id) {
                if let Some(portal) = ctx.db.portal_state().entity_id().find(location.entity_id) {
                    if portal.destination_dimension == outside_dimension {
                        return portal.entity_id;
                    }
                }
            }
        }
        panic!("Could not find portal leading to exterior in player housing");
    }

    pub fn get_rank_and_template_building(ctx: &ReducerContext, actor_id: u64) -> (i32, i32) {
        let mut highest_rank = 0;
        let mut template_building_id = 0;

        for player_housing in ctx.db.player_housing_desc().iter() {
            if Discovery::already_acquired_secondary(ctx, actor_id, player_housing.secondary_knowledge_id) {
                highest_rank = player_housing.rank.max(highest_rank);
                template_building_id = player_housing.template_building_id;
            }
        }
        (highest_rank, template_building_id)
    }
}
