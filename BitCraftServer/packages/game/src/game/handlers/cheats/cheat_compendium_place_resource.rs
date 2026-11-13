use crate::game::game_state::game_state_filters;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::game::reducer_helpers::footprint_helpers::clear_and_flatten_terrain_under_footprint;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::action_request::CheatCompendiumItemPlaceRequest;
use crate::messages::components::ResourceState;
use crate::resource_desc;

use spacetimedb::ReducerContext;

// TODO: Refactor to share the logic with lib.rs -> fn commit_generated_world
// Similar to: build.rs, lib.rs,
#[spacetimedb::reducer]
pub fn cheat_compendium_place_resource(ctx: &ReducerContext, request: CheatCompendiumItemPlaceRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatCompendiumPlaceResource) {
        return Err("Unauthorized.".into());
    }

    if let Some(resource_desc) = ctx.db.resource_desc().id().find(request.item_id) {
        let coordinates = request.coordinates.into();
        let mut terrain_cache = TerrainChunkCache::empty();
        let should_flatten_terrain = !game_state_filters::is_submerged(ctx, &mut terrain_cache, coordinates);
        let footprint = resource_desc.get_footprint(&coordinates, request.facing_direction);
        clear_and_flatten_terrain_under_footprint(ctx, &footprint, should_flatten_terrain);

        ResourceState::spawn(
            ctx,
            None,
            resource_desc.id,
            coordinates,
            request.facing_direction,
            ctx.db.resource_desc().id().find(resource_desc.id).unwrap().max_health,
            false,
            false,
        )?;
    } else {
        return Err(format!("[Cheat] Failed to create resource from item_id {}", request.item_id));
    }

    Ok(())
}
