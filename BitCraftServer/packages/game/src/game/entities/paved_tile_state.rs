use std::collections::HashMap;

use spacetimedb::{ReducerContext, Table};

use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::{coordinates::*, game_state};
use crate::messages::game_util::ItemStack;
use crate::messages::{
    components::{LocationState, PavedTileState},
    static_data::*,
};
use crate::{location_state, paved_tile_state};

use super::building_state::InventoryState;

impl PavedTileState {
    pub fn get_at_location(ctx: &ReducerContext, coordinates: &SmallHexTile) -> Option<PavedTileState> {
        LocationState::select_all(ctx, coordinates)
            .filter_map(|ls| ctx.db.paved_tile_state().entity_id().find(ls.entity_id))
            .next()
    }

    pub fn collect_stats(ctx: &ReducerContext, coordinates: &SmallHexTile, bonuses: &mut HashMap<CharacterStatType, (f32, f32)>) {
        if let Some(paved_tile) = Self::get_at_location(ctx, coordinates) {
            let paving_info = ctx.db.paving_tile_desc().id().find(paved_tile.tile_type_id).unwrap();
            for CsvStatEntry { id, value, is_pct } in &paving_info.stat_effects {
                let entry = bonuses.entry(*id).or_insert((0.0, 0.0));
                if *is_pct {
                    *entry = (entry.0, entry.1 + value);
                } else {
                    *entry = (entry.0 + value, entry.1);
                }
            }
        }
    }

    pub fn delete_paving(ctx: &ReducerContext, entity_id: &u64) {
        ctx.db.location_state().entity_id().delete(entity_id);
        ctx.db.paved_tile_state().entity_id().delete(entity_id);
    }

    pub fn refund_paving(ctx: &ReducerContext, paving: &PavedTileState, inventory: &mut InventoryState) {
        //Refund materials
        let tile_description = match ctx.db.paving_tile_desc().id().find(paving.tile_type_id) {
            Some(t) => t,
            None => return, //Paving type no longer exists, nothing to refund
        };

        let mut consumed_item_stacks: Vec<ItemStack> = tile_description
            .consumed_item_stacks
            .iter()
            .map(|i| ItemStack::new(ctx, i.item_id, i.item_type, i.quantity))
            .collect();

        if tile_description.input_cargo_id != 0 {
            consumed_item_stacks.push(ItemStack::new(
                ctx,
                tile_description.input_cargo_id,
                crate::messages::game_util::ItemType::Cargo,
                1,
            ));
        }

        if !consumed_item_stacks.is_empty() {
            inventory.add_multiple_with_overflow(ctx, &consumed_item_stacks);
        }
    }

    pub fn create_paving_unsafe(
        ctx: &ReducerContext,
        coordinates: SmallHexTile,
        tile_type_id: i32,
        related_entity_id: u64,
    ) -> Result<(), String> {
        let mut terrain_cache = TerrainChunkCache::empty();

        // Verify distance to paving target
        let target_coord = coordinates;

        if terrain_cache.get_terrain_cell(ctx, &target_coord.parent_large_tile()).is_none() {
            return Err("Invalid coordinates".into());
        }

        if !game_state::game_state_filters::is_flat_corner(ctx, &mut terrain_cache, target_coord) {
            return Err("Can only pave flat terrain".into());
        }

        // Delete existing paving
        let existing_paving = PavedTileState::get_at_location(ctx, &target_coord);

        if let Some(ref paving) = existing_paving {
            if paving.tile_type_id == tile_type_id {
                return Err("This tile already has that type of pavement".into());
            }
        }

        // Delete existing paving
        if let Some(paving) = existing_paving {
            PavedTileState::delete_paving(ctx, &paving.entity_id);
        }

        // Create paved tile
        let entity_id = game_state::create_entity(ctx);

        // location
        let offset = target_coord.to_offset_coordinates();
        game_state::insert_location(ctx, entity_id, offset);

        // tile entity
        let paved_tile = PavedTileState {
            entity_id,
            tile_type_id,
            related_entity_id,
        };

        if ctx.db.paved_tile_state().try_insert(paved_tile).is_err() {
            return Err("Failed to insert pavement".into());
        }

        // Previously despawned resources under paving, this was removed because it was unintentionally removing resources that were part of the resource spawns

        Ok(())
    }
}
