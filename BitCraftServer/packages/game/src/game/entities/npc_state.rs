use std::time::Duration;

use spacetimedb::rand::{seq::IteratorRandom, Rng};
use spacetimedb::{log, ReducerContext, Table, TimeDuration, Timestamp};

use crate::game::reducer_helpers::building_helpers::create_building_unsafe;
use crate::{
    building_state, location_state, mobile_entity_state, npc_desc, npc_state, parameters_desc, trade_order_state,
    traveler_trade_order_desc, NpcType,
};
use crate::{
    game::game_state,
    messages::{
        components::{NpcState, TradeOrderState},
        static_data::BuildingSpawnDesc,
        util::OffsetCoordinatesFloat,
    },
};

use super::{building_state::BuildingState, location::MobileEntityState, resource_clump::OffsetCoordinatesSmall};

impl NpcState {
    pub fn spawn(
        ctx: &ReducerContext,
        traveler_type: NpcType,
        direction: i32,
        ruin_entity_id: u64,
        offset_coord: OffsetCoordinatesSmall,
        traveling: bool,
    ) {
        let npc_entity_id = game_state::create_entity(ctx);
        let npc = NpcState {
            entity_id: npc_entity_id,
            npc_type: traveler_type,
            direction,
            building_entity_id: ruin_entity_id,
            next_action_timestamp: Self::get_next_timestamp(ctx, traveler_type),
            move_duration: 0.0,
            started_moving: 0,
            previous_buildings: Vec::new(),
            traveling,
        };
        log::info!("Spawning {:?} Npc at offset coord {:?}", traveler_type, offset_coord);
        game_state::insert_location_float(ctx, npc_entity_id, offset_coord.into());
        npc.create_trade_orders(ctx);
        if let Err(e) = ctx.db.npc_state().try_insert(npc) {
            panic!("{}", e);
        }
    }

    pub fn spawn_with_ruins(
        ctx: &ReducerContext,
        traveler_type: NpcType,
        ruin_description_id: i32,
        direction: i32,
        offset_coord: OffsetCoordinatesSmall,
        traveling: bool,
    ) {
        // add a matching traveler's ruins
        let ruin_entity_id = game_state::create_entity(ctx);
        let building_desc_id = ruin_description_id;
        create_building_unsafe(ctx, 0, Some(ruin_entity_id), offset_coord.into(), direction, building_desc_id, None).unwrap();
        Self::spawn(ctx, traveler_type, direction, ruin_entity_id, offset_coord, traveling);
    }

    pub fn create_trade_orders(&self, ctx: &ReducerContext) {
        let building_entity_id = self.building_entity_id;
        let traveler_type = self.npc_type;

        // add "always_offered" trade orders at new location
        let mut ids = Vec::new();
        for traveler_trade_order in ctx.db.traveler_trade_order_desc().iter() {
            if traveler_trade_order.traveler == traveler_type {
                if traveler_trade_order.always_offered {
                    TradeOrderState::create(
                        ctx,
                        building_entity_id,
                        traveler_trade_order.starting_stock,
                        &traveler_trade_order.offer_items,
                        &traveler_trade_order.required_items,
                        traveler_trade_order.id.into(),
                    );
                } else {
                    ids.push(traveler_trade_order.id);
                }
            }
        }

        // sample a subset of trade orders from the set that is not "always_offered"
        let mut rng = ctx.rng();
        let sampled_ids = ids.iter().choose_multiple(
            &mut rng,
            ctx.db.parameters_desc().version().find(0).unwrap().selected_traveler_order_count as usize,
        );
        for id in sampled_ids {
            let traveler_trade_order = ctx.db.traveler_trade_order_desc().id().find(id).unwrap();
            TradeOrderState::create(
                ctx,
                building_entity_id,
                traveler_trade_order.starting_stock,
                &traveler_trade_order.offer_items,
                &traveler_trade_order.required_items,
                (*id).into(),
            );
        }
    }

    pub fn get_next_timestamp(ctx: &ReducerContext, npc_type: NpcType) -> Timestamp {
        let npc_type = npc_type as i32;
        let npc_desc = ctx.db.npc_desc().npc_type().find(npc_type).unwrap();
        let delay = ctx.rng().gen_range(npc_desc.min_time_at_ruin..npc_desc.max_time_at_ruin) as u64;
        let duration = Duration::from_secs(delay);
        ctx.timestamp + TimeDuration::from(duration)
    }

    pub fn delete_trade_orders(&self, ctx: &ReducerContext) {
        let orders_to_delete = ctx.db.trade_order_state().shop_entity_id().filter(self.building_entity_id);
        for order in orders_to_delete {
            ctx.db.trade_order_state().entity_id().delete(order.entity_id);
        }
    }

    pub fn teleport(&mut self, ctx: &ReducerContext, building: &BuildingState) {
        // Keep track of the last 3 ruins visited
        if self.building_entity_id != 0 {
            self.previous_buildings.push(self.building_entity_id);
            while self.previous_buildings.len() > 3 {
                self.previous_buildings.remove(0);
            }
        }

        // delete current trade orders
        self.delete_trade_orders(ctx);

        self.building_entity_id = building.entity_id;
        self.next_action_timestamp = NpcState::get_next_timestamp(ctx, self.npc_type);

        let building_coordinates = ctx.db.location_state().entity_id().find(building.entity_id).unwrap().coordinates();

        self.direction = BuildingSpawnDesc::get_traveler_direction(ctx, building.building_description_id, building.direction_index);

        let dest_coords = BuildingSpawnDesc::get_traveler_spawn_coordinates(
            ctx,
            building.building_description_id,
            &building_coordinates,
            building.direction_index,
        );
        let offset = OffsetCoordinatesSmall::from(dest_coords);
        let offset_float = OffsetCoordinatesFloat::from(offset);

        let npc_location = MobileEntityState::for_location(self.entity_id, offset_float, ctx.timestamp);

        ctx.db.mobile_entity_state().entity_id().update(npc_location);

        // create new trade orders
        self.create_trade_orders(ctx);
    }

    pub fn refresh_all_traveler_trade_orders(ctx: &ReducerContext) {
        for npc in ctx.db.npc_state().iter() {
            if ctx.db.building_state().entity_id().find(npc.building_entity_id).is_none() {
                continue;
            }

            npc.delete_trade_orders(ctx);
            npc.create_trade_orders(ctx);
        }
    }
}
