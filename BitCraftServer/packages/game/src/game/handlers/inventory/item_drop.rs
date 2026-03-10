use crate::game::game_state::{self, game_state_filters};
use crate::game::permission_helper;
use crate::game::reducer_helpers::loot_chest_helpers;
use crate::game::reducer_helpers::player_action_helpers::post_reducer_update_cargo;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::action_request::PlayerItemDropRequest;
use crate::messages::components::*;
use crate::{unwrap_or_err, FloatHexTile, FootprintType, SmallHexTile};
use spacetimedb::ReducerContext;

use super::inventory_helper;

#[spacetimedb::reducer]
pub fn item_drop(ctx: &ReducerContext, request: PlayerItemDropRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let player_coordinates_float = game_state_filters::coordinates_float(ctx, actor_id);
    let mut pile_coordinates = player_coordinates_float.parent_small_tile();

    let item_stack;

    if request.pocket.inventory_entity_id == 0 {
        // Equipment slot
        let mut equipment = unwrap_or_err!(ctx.db.equipment_state().entity_id().find(actor_id), "Invalid Equipment State");
        if let Some(slot) = equipment.equipment_slots.get(request.pocket.pocket_index as usize) {
            if let Some(item) = slot.item {
                item_stack = item;
                equipment.equipment_slots[request.pocket.pocket_index as usize].item = None;
                ctx.db.equipment_state().entity_id().update(equipment);
                PlayerState::collect_stats(ctx, actor_id);
            } else {
                return Err("Nothing equipped on this slot".into());
            }
        } else {
            return Err("Invalid equipment slot".into());
        }
    } else {
        // Inventory slot
        let mut source_inventory = unwrap_or_err!(
            ctx.db.inventory_state().entity_id().find(&request.pocket.inventory_entity_id),
            "Invalid source inventory"
        );
        let source_inventory_type = inventory_helper::validate_interact(
            ctx,
            actor_id,
            pile_coordinates,
            source_inventory.owner_entity_id,
            source_inventory.player_owner_entity_id,
        )?;

        // Ensure this inventory type is allowed as a drop source.
        inventory_helper::validate_drop(&source_inventory_type)?;

        let pocket_index: usize = request.pocket.pocket_index as usize;
        item_stack = unwrap_or_err!(
            source_inventory.get_pocket_contents(pocket_index),
            "Failed to drop items, no pocket exists at the requested pocket index!"
        );

        source_inventory.remove_at(pocket_index);

        // Dropping from toolbelt
        if source_inventory.owner_entity_id == actor_id && source_inventory.inventory_index == 1 {
            PlayerState::on_removed_from_toolbelt(ctx, actor_id, item_stack.item_id);
        }

        if source_inventory.is_empty() {
            if ctx
                .db
                .loot_chest_state()
                .entity_id()
                .find(&source_inventory.owner_entity_id)
                .is_some()
            {
                loot_chest_helpers::on_item_taken_from_loot_chest(ctx, source_inventory.owner_entity_id, true)?;
            }
        }

        source_inventory.update(ctx);
    }

    //Try to avoid creating item piles on uneven corners
    if pile_coordinates.is_corner() {
        let center = pile_coordinates;
        let mut terrain_cache = TerrainChunkCache::empty();
        if !game_state_filters::is_flat_corner(ctx, &mut terrain_cache, center) {
            let player_elevation = terrain_cache
                .get_terrain_cell(ctx, &player_coordinates_float.parent_large_tile())
                .unwrap()
                .elevation;

            let mut neighbors: Vec<SmallHexTile> = center.neighbors_direct().collect();
            neighbors.sort_by_key(|t| (FloatHexTile::from(t).distance_to(player_coordinates_float) * 1000.0) as i32);
            for neighbor in neighbors {
                if !PermissionState::can_interact_with_tile(ctx, actor_id, neighbor, Permission::Usage) {
                    continue;
                }

                if !permission_helper::can_interact_with_tile(ctx, neighbor, actor_id, ClaimPermission::Usage) {
                    continue;
                }

                if FootprintTileState::get_at_location(ctx, &neighbor)
                    .any(|fp| fp.footprint_type == FootprintType::Hitbox || fp.footprint_type == FootprintType::Walkable)
                {
                    continue;
                }

                let neighbor_elevation = terrain_cache
                    .get_terrain_cell(ctx, &neighbor.parent_large_tile())
                    .unwrap()
                    .elevation;
                if (neighbor_elevation - player_elevation).abs() > 4 {
                    continue;
                }

                pile_coordinates = neighbor;
                break;
            }
        }
    }

    DroppedInventoryState::update_from_items(ctx, actor_id, pile_coordinates, vec![item_stack], None);

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
