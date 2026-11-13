use spacetimedb::{log, ReducerContext, Table, Timestamp};

pub use crate::game::coordinates::*;
use crate::game::game_state::{self, game_state_filters};
use crate::game::reducer_helpers::timer_helpers::now_plus_secs;
use crate::messages::authentication::ServerIdentity;
use crate::messages::components::*;
use crate::messages::game_util::ItemStack;
use crate::messages::util::SmallHexTileMessage;
use crate::{parameters_desc, ItemListDesc};

#[spacetimedb::table(accessor = dropped_inventory_ownership_timer, public, scheduled(dropped_inventory_lose_ownership, at = scheduled_at), 
    index(accessor = entity_id, btree(columns = [entity_id])))]
pub struct DroppedInventoryOwnershipTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
    pub started_at: Timestamp,
}

#[spacetimedb::reducer]
pub fn dropped_inventory_lose_ownership(ctx: &ReducerContext, timer: DroppedInventoryOwnershipTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to dropped_inventory_lose_ownership");
        return;
    }
    
    // Ownership loss timer converts into a despawn timer (if the dropped inventory still exists)
    if let Some(mut dropped_inventory) = ctx.db.dropped_inventory_state().entity_id().find(timer.entity_id) {

        let coord = game_state_filters::coordinates(ctx, timer.entity_id);

        if DroppedInventoryState::get_at_location(ctx, 0, coord).is_some() {
            // Merge with existing public dropped inventory
            let inventory = ctx.db.inventory_state().entity_id().find(timer.entity_id).unwrap();
            
            let item_stacks: Vec<ItemStack> = inventory.pockets.iter().filter_map( |p| p.contents ).collect();           
            DroppedInventoryState::update_from_items(ctx, 0, coord, item_stacks, None);
            dropped_inventory.delete(ctx);

        } else {
            dropped_inventory.insert_despawn_timer(ctx);
            dropped_inventory.update(ctx);               
        }
    }
}

#[spacetimedb::table(accessor = dropped_inventory_despawn_timer, public, scheduled(dropped_inventory_despawn, at = scheduled_at),
    index(accessor = entity_id, btree(columns = [entity_id])))]
pub struct DroppedInventoryDespawnTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
    pub started_at: Timestamp,
}

#[spacetimedb::reducer]
pub fn dropped_inventory_despawn(ctx: &ReducerContext, timer: DroppedInventoryDespawnTimer) {
    if ServerIdentity::validate_server_or_admin(ctx).is_err() {
        log::error!("Unauthorized access to dropped_inventory_despawn");
        return;
    }
    // Despawn the dropped inventory if it still exists
    if let Some(dropped_inventory) = ctx.db.dropped_inventory_state().entity_id().find(timer.entity_id) {
        dropped_inventory.delete(ctx);
    }
}

impl DroppedInventoryState {
    pub fn update(self, ctx: &ReducerContext) {
        ctx.db.dropped_inventory_state().entity_id().update(self);
    }

    pub fn insert(self, ctx: &ReducerContext) {
        ctx.db.dropped_inventory_state().insert(self);
    }

    pub fn new(ctx: &ReducerContext, owner_entity_id: u64) -> Self {
        let entity_id = game_state::create_entity(ctx);

        DroppedInventoryState {
            entity_id,
            owner_entity_id,
            active_timer_id: 0,
        }
    }

    pub fn inventory(&self, ctx: &ReducerContext) -> InventoryState {
        ctx.db.inventory_state().entity_id().find(self.entity_id).unwrap()
    }

    pub fn delete(&self, ctx: &ReducerContext) {
        let entity_id = self.entity_id;
        ctx.db.dropped_inventory_state().entity_id().delete(entity_id);
        ctx.db.location_state().entity_id().delete(entity_id);
        ctx.db.inventory_state().entity_id().delete(entity_id);
        self.delete_previous_timer(ctx);
    }

    fn delete_previous_timer(&self, ctx: &ReducerContext) {
        if self.active_timer_id != 0 {
            if self.owner_entity_id == 0 {
                ctx.db.dropped_inventory_despawn_timer().scheduled_id().delete(self.active_timer_id);               
            } else {
                ctx.db
                .dropped_inventory_ownership_timer()
                .scheduled_id()
                .delete(self.active_timer_id);
            }
        }
    }

    pub fn new_inventory(&self) -> InventoryState {
        InventoryState::create_with_pockets(self.entity_id, 50, i32::MAX, i32::MAX, 0, 30, self.entity_id, 0, None)
    }

    pub fn insert_ownership_timer(&mut self, ctx: &ReducerContext, actor_id: u64, ownership_duration: Option<i32>) {

        if actor_id == 0 {
            // A no-ownership timer is the same as a despawn timer refresh
            self.insert_despawn_timer(ctx);
            return;
        }

        let duration_sec = ownership_duration.unwrap_or(
            ctx
            .db
            .parameters_desc()
            .version()
            .find(0)
            .unwrap()
            .dropped_inventory_ownership_seconds) as u64;

        self.delete_previous_timer(ctx);

        let timer = ctx.db.dropped_inventory_ownership_timer().insert(DroppedInventoryOwnershipTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(duration_sec, ctx.timestamp),
            entity_id: self.entity_id,
            started_at: ctx.timestamp,
        });
        self.owner_entity_id = actor_id;
        self.active_timer_id = timer.scheduled_id;
    }

    pub fn insert_despawn_timer(&mut self, ctx: &ReducerContext) {
        let duration_sec = ctx
            .db
            .parameters_desc()
            .version()
            .find(0)
            .unwrap()
            .dropped_inventory_despawn_seconds as u64;

        self.delete_previous_timer(ctx);
        self.owner_entity_id = 0;

        let timer = ctx.db.dropped_inventory_despawn_timer().insert(DroppedInventoryDespawnTimer {
            scheduled_id: 0,
            scheduled_at: now_plus_secs(duration_sec, ctx.timestamp),
            entity_id: self.entity_id,
            started_at: ctx.timestamp,
        });

        self.active_timer_id = timer.scheduled_id;
    }

    pub fn get_at_location(ctx: &ReducerContext, actor_id: u64, loc: SmallHexTile) -> Option<DroppedInventoryState> {
        let mut public_dropped_inventory = None;

        // Find public and actor_id private dropped inventories on this tile
        for dropped_inventory in LocationState::select_all(ctx, &loc)
            .filter_map(|loc| ctx.db.dropped_inventory_state().entity_id().find(loc.entity_id))
            .filter(|di| di.owner_entity_id == 0 || di.owner_entity_id == actor_id)
        {
            // Private dropped inventories have priority and are returned immediately
            if dropped_inventory.owner_entity_id == actor_id {
                return Some(dropped_inventory);
            }
            // A good candidate if we don't find a private dropped inventory
            public_dropped_inventory = Some(dropped_inventory);
        }
        // Either no dropped inventory or a public dropped inventory
        public_dropped_inventory
    }

    pub fn update_from_items(ctx: &ReducerContext, actor_id: u64, loc: SmallHexTile, item_stacks: Vec<ItemStack>, ownership_duration: Option<i32>) {
        
        if item_stacks.is_empty() {
            return;
        }

        let mut dropped_inventory;
        let mut inventory;
        let is_new_inventory;
        if let Some(inv) = DroppedInventoryState::get_at_location(ctx, actor_id, loc) {
            is_new_inventory = false;
            inventory = ctx.db.inventory_state().entity_id().find(inv.entity_id).unwrap();
            dropped_inventory = inv;
        } else {
            is_new_inventory = true;
            dropped_inventory = DroppedInventoryState::new(ctx, actor_id);
            inventory = dropped_inventory.new_inventory();
            game_state::insert_location(ctx, dropped_inventory.entity_id, loc.into());
        }
        dropped_inventory.insert_ownership_timer(ctx, actor_id, ownership_duration);

        for item_stack in item_stacks {
            let converted_items = ItemListDesc::extract_item_stacks_from_item(ctx, item_stack, Some(actor_id));
            while !inventory.add_multiple(ctx, &converted_items) {
                // Double inventory capacity
                inventory.double();
            }
        }

        if is_new_inventory {
            dropped_inventory.insert(ctx);
            inventory.insert(ctx);
        } else {
            dropped_inventory.update(ctx);
            inventory.update(ctx);
        }
    }

    pub fn validate_ownership(&self, actor_id: u64) -> Result<(), String> {
        let owner = self.owner_entity_id;
        if owner != 0 && owner != actor_id {
            return Err("You are not the owner of those items".into());
        }
        Ok(())
    }

    pub fn validate_interact_and_get_inventory_coordinates(&self, ctx: &ReducerContext, actor_id: u64) -> Result<SmallHexTileMessage, String> {
        self.validate_ownership(actor_id)?;

        let dropped_inventory_coordinates = game_state_filters::coordinates_any(ctx, self.entity_id);
        let player_coordinates = game_state_filters::coordinates_any(ctx, actor_id);
    
        if dropped_inventory_coordinates.distance_to(player_coordinates) > 3 {
            return Err("Too far".into());
        }
    
        if self.owner_entity_id != 0 && self.owner_entity_id != actor_id {
            return Err("You don't have permission to pick this item up".into());
        }

        Ok(dropped_inventory_coordinates)    
    }

    pub fn on_inventory_updated(mut self, ctx: &ReducerContext, actor_id: u64, self_inventory: InventoryState) {
        if self_inventory.is_empty() {
            ctx.db.inventory_state().entity_id().delete(self_inventory.entity_id);
            self.delete(ctx);
        } else {
            self.insert_ownership_timer(ctx, actor_id, None);
            self.update(ctx);
            self_inventory.update(ctx);
        }
    }
}
