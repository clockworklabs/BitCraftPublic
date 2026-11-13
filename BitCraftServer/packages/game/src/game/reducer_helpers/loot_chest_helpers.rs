use spacetimedb::{log, rand::Rng};
use spacetimedb::{ReducerContext, Table};

use super::{footprint_helpers, interior_helpers::interior_trigger_collapse, timer_helpers::now_plus_secs_f32};

use crate::game::handlers::server::loot_chest_despawn::loot_chest_despawn_timer;
use crate::messages::components::{FootprintTileState, LootChestState};
use crate::{
    chest_rarity_desc, footprint_tile_state, interior_collapse_trigger_state, location_state, loot_chest_desc, loot_chest_state,
    parameters_desc,
};
use crate::{
    game::{
        coordinates::SmallHexTile,
        game_state::{self, game_state_filters},
        handlers::server::loot_chest_despawn::LootChestDespawnTimer,
    },
    messages::{game_util::ItemStack, static_data::FootprintType},
    unwrap_or_err, InventoryState,
};

pub fn delete_loot_chest(ctx: &ReducerContext, entity_id: u64) {
    footprint_helpers::delete_footprint(ctx, entity_id);
    game_state_filters::remove_entity_inventories(ctx, entity_id);
    ctx.db.loot_chest_state().entity_id().delete(entity_id);
    ctx.db.location_state().entity_id().delete(entity_id);
}

#[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
pub fn spawn_loot_chest(
    ctx: &ReducerContext,
    loot_chests: &[i32],
    entity_id: u64,
    spawn_location: SmallHexTile,
    direction_index: i32,
    building_entity_id: u64,
    building_spawn_id: i32,
    verbose: bool,
) -> Result<(), String> {
    let loot_chest_ix = ctx.rng().gen_range(0..loot_chests.len());

    let loot_chest_desc_id = loot_chests[loot_chest_ix];

    let loot_chest_desc = unwrap_or_err!(ctx.db.loot_chest_desc().id().find(loot_chest_desc_id), "Failed to find loot table");

    let chest_rarity_desc = unwrap_or_err!(
        ctx.db.chest_rarity_desc().id().find(loot_chest_desc.chest_rarity),
        "Failed to find chest rarity"
    );

    if let Some(loot_table) = chest_rarity_desc.roll(ctx, &loot_chest_desc.loot_tables, verbose) {
        if verbose {
            log::debug!("Loot chest spawning at {}, {}", spawn_location.x, spawn_location.z,);
        }

        let item_stacks: Vec<ItemStack> = loot_table.roll(ctx, verbose);

        if item_stacks.is_empty() {
            log::debug!("Loot table roll failed.");
            return Err("Failed to generate any loot".into());
        }

        let offset_coords = spawn_location.to_offset_coordinates();

        game_state::insert_location(ctx, entity_id, offset_coords);
        if ctx
            .db
            .loot_chest_state()
            .try_insert(LootChestState {
                entity_id,
                loot_chest_id: loot_chest_desc_id,
                building_entity_id,
                direction_index,
                building_spawn_id,
            })
            .is_err()
        {
            return Err("Failed to insert loot chest".into());
        }

        let footprint_entity_id = game_state::create_entity(ctx);
        if ctx
            .db
            .footprint_tile_state()
            .try_insert(FootprintTileState {
                entity_id: footprint_entity_id,
                footprint_type: FootprintType::Hitbox,
                owner_entity_id: entity_id,
            })
            .is_err()
        {
            return Err("Failed to insert footprint".into());
        }
        game_state::insert_location(ctx, footprint_entity_id, offset_coords);

        if !InventoryState::new(ctx, 15, 6000, 6000, 15, entity_id, 0, Some(item_stacks)) {
            return Err("Failed to insert inventory state".into());
        }
    } else {
        return Err("Failed to generate any loot".into());
    }

    Ok(())
}

pub fn on_item_taken_from_loot_chest(ctx: &ReducerContext, owner_entity_id: u64, from_inventory_empty: bool) -> Result<(), String> {
    let loot_chest = ctx.db.loot_chest_state().entity_id().find(owner_entity_id).unwrap();
    if loot_chest.building_entity_id != 0 {
        //Chest in overworld
        // if this item was removed from a loot chest, then start a timer to re-roll it
        if from_inventory_empty {
            let loot_chest = ctx.db.loot_chest_state().entity_id().find(owner_entity_id).unwrap();
            let despawn_time = ctx.db.parameters_desc().version().find(0).unwrap().loot_chest_despawn_time_seconds;
            ctx.db
                .loot_chest_despawn_timer()
                .try_insert(LootChestDespawnTimer {
                    scheduled_id: 0,
                    scheduled_at: now_plus_secs_f32(despawn_time, ctx.timestamp),
                    loot_chest_entity_id: owner_entity_id,
                    should_respawn: loot_chest.building_spawn_id >= 0,
                })
                .ok()
                .unwrap();
        }
    } else {
        if from_inventory_empty {
            //Chest inside ancient ruin
            ctx.db
                .loot_chest_despawn_timer()
                .try_insert(LootChestDespawnTimer {
                    scheduled_id: 0,
                    scheduled_at: ctx.timestamp.into(),
                    loot_chest_entity_id: owner_entity_id,
                    should_respawn: false,
                })
                .ok()
                .unwrap();
        }

        if let Some(collapse_trigger) = ctx.db.interior_collapse_trigger_state().entity_id().find(owner_entity_id) {
            return interior_trigger_collapse(ctx, collapse_trigger.dimension_network_entity_id);
        }
    }

    Ok(())
}
