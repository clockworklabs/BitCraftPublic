use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::{footprint_tile_state, loot_chest_state};
use spacetimedb::{ReducerContext, Table};

use crate::game::{coordinates::*, game_state};
use crate::{
    game::entities::building_state::InventoryState,
    messages::{
        action_request::CheatSpawnLootChestRequest,
        components::{FootprintTileState, LootChestState},
        game_util::ItemStack,
        static_data::*,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
fn cheat_spawn_loot_chest(ctx: &ReducerContext, request: CheatSpawnLootChestRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatSpawnLootChest) {
        return Err("Unauthorized.".into());
    }

    reduce(ctx, request.coordinates.into(), request.loot_chest_id)
}

pub fn reduce(ctx: &ReducerContext, spawn_location: SmallHexTile, loot_chest_desc_id: i32) -> Result<(), String> {
    let loot_chest_desc = unwrap_or_err!(ctx.db.loot_chest_desc().id().find(loot_chest_desc_id), "Failed to find loot table");

    let chest_rarity_desc = unwrap_or_err!(
        ctx.db.chest_rarity_desc().id().find(loot_chest_desc.chest_rarity),
        "Failed to find chest rarity"
    );

    if let Some(loot_table) = chest_rarity_desc.roll(ctx, &loot_chest_desc.loot_tables, true) {
        spacetimedb::log::debug!("Loot chest spawning at {}, {}", spawn_location.x, spawn_location.z,);

        let item_stacks: Vec<ItemStack> = loot_table.roll(ctx, true);

        if item_stacks.is_empty() {
            spacetimedb::log::debug!("Loot table roll failed.");
            return Err("Failed to generate any loot".into());
        }

        let entity_id = game_state::create_entity(ctx);

        let offset_coords = OffsetCoordinatesSmall::from(spawn_location);

        game_state::insert_location(ctx, entity_id, offset_coords);
        ctx.db
            .loot_chest_state()
            .try_insert(LootChestState {
                entity_id,
                loot_chest_id: loot_chest_desc_id,
                building_entity_id: 0,
                direction_index: 0,
                building_spawn_id: -1,
            })
            .unwrap();

        let footprint_entity_id = game_state::create_entity(ctx);
        ctx.db
            .footprint_tile_state()
            .try_insert(FootprintTileState {
                entity_id: footprint_entity_id,
                footprint_type: FootprintType::Hitbox,
                owner_entity_id: entity_id,
            })
            .unwrap();
        game_state::insert_location(ctx, footprint_entity_id, offset_coords);

        if !InventoryState::new(ctx, 15, 6000, 6000, 15, entity_id, 0, Some(item_stacks)) {
            return Err("Failed to insert inventory state".into());
        }
    } else {
        return Err("Failed to generate any loot".into());
    }

    Ok(())
}
