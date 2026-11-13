use crate::game::coordinates::{HexDirection, SmallHexTile};
use crate::game::game_state;
use crate::game::game_state::game_state_filters;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::reducer_helpers::building_helpers::{create_building_unsafe, setup_npc_watchtower_state};
use crate::game::reducer_helpers::footprint_helpers::clear_and_flatten_terrain_under_footprint;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::action_request::PlayerProjectSitePlaceRequest;
use crate::messages::components::user_state;
use crate::messages::static_data::{building_desc, building_spawn_desc, resource_desc, FootprintType};
use crate::{construction_recipe_desc, unwrap_or_err};

use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

const DEFAULT_NPC_WATCHTOWER_ENERGY: i32 = 10000;
const DEFAULT_NPC_WATCHTOWER_UPKEEP: i32 = 2;

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn cheat_building_place(ctx: &ReducerContext, request: PlayerProjectSitePlaceRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatBuildingPlace) {
        return Err("Unauthorized.".into());
    }

    // from build.rs -> pub reduce()
    let recipe = unwrap_or_err!(
        ctx.db.construction_recipe_desc().id().find(request.construction_recipe_id),
        "Invalid recipe."
    );

    let coord = SmallHexTile::from(request.coordinates);
    let actor_id = match ctx.db.user_state().identity().find(ctx.sender()) {
        Some(user) => user.entity_id,
        None => 0,
    };

    let building_desc = ctx.db.building_desc().id().find(recipe.building_description_id).unwrap();
    let mut terrain_cache = TerrainChunkCache::empty();
    let should_flatten_terrain = !game_state_filters::is_submerged(ctx, &mut terrain_cache, coord);
    let footprint = building_desc.get_footprint(&coord, request.facing_direction);
    clear_and_flatten_terrain_under_footprint(ctx, &footprint, should_flatten_terrain);

    // clear and flatten terrain under footprint of all building spawns:
    for building_spawn in ctx.db.building_spawn_desc().building_id().filter(building_desc.id) {
        let building_direction = HexDirection::from(request.facing_direction);
        let spawn_direction = (HexDirection::from(building_spawn.direction) + building_direction) as i32;
        let spawn_coord = building_spawn.get_spawn_coordinates(&coord, request.facing_direction);
        match building_spawn.spawn_type {
            crate::messages::static_data::BuildingSpawnType::Building => {
                let building_desc = ctx.db.building_desc().id().find(building_spawn.spawn_ids[0]).unwrap();
                let footprint = building_desc.get_footprint(&spawn_coord, spawn_direction);
                clear_and_flatten_terrain_under_footprint(ctx, &footprint, should_flatten_terrain);
            }
            crate::messages::static_data::BuildingSpawnType::Resource => {
                let resource_desc = ctx.db.resource_desc().id().find(building_spawn.spawn_ids[0]).unwrap();
                let footprint = resource_desc.get_footprint(&spawn_coord, spawn_direction);
                clear_and_flatten_terrain_under_footprint(ctx, &footprint, should_flatten_terrain);
            }
            crate::messages::static_data::BuildingSpawnType::Paving => {
                let footprint = vec![(spawn_coord, FootprintType::Hitbox)];
                clear_and_flatten_terrain_under_footprint(ctx, &footprint, should_flatten_terrain);
            }
            _ => continue,
        }
    }

    let entity_id = game_state::create_entity(ctx);

    create_building_unsafe(
        ctx,
        actor_id,
        Some(entity_id),
        coord,
        request.facing_direction,
        recipe.building_description_id,
        None,
    )?;

    setup_npc_watchtower_state(
        ctx,
        entity_id,
        &building_desc,
        coord,
        DEFAULT_NPC_WATCHTOWER_ENERGY,
        DEFAULT_NPC_WATCHTOWER_UPKEEP,
    );

    Ok(())
}
