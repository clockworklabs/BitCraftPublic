use bitcraft_macro::feature_gate;
use spacetimedb::{log, rand::Rng, ReducerContext};

use crate::{
    agents::resources_regen::{self, LazyMemoOccupiedTiles},
    game::{
        handlers::authentication::has_role,
        terrain_chunk::TerrainChunkCache,
        world_gen::resources_log::{resources_log, single_resource_clump_info},
    },
    messages::{authentication::Role, static_data::resource_clump_desc},
    parameters_desc, resource_desc, resource_state, terrain_chunk_state, unwrap_or_err, OffsetCoordinatesSmall, ResourceState,
    SmallHexTile, TerrainChunkState,
};

#[spacetimedb::table(accessor = respawn_resource_in_chunk_timer, scheduled(respawn_resource_in_chunk, at = scheduled_at))]
pub struct RespawnResourceInChunkTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub chunk_index: u64,
    pub resource_clump_id: i32,
    pub coord: SmallHexTile,
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn respawn_resource_in_chunk(ctx: &ReducerContext, timer: RespawnResourceInChunkTimer) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let resources_log = unwrap_or_err!(ctx.db.resources_log().version().find(0), "Failed to get resources_log");
    let resource_clump = unwrap_or_err!(
        ctx.db.resource_clump_desc().id().find(timer.resource_clump_id),
        "Failed to get resource_clump_desc"
    );

    //Don't spawn this clump if any resources in the clump have a world target of 0
    if resource_clump.resource_id.iter().any(|x| {
        resources_log
            .resources
            .iter()
            .find(|y| y.resource_id == *x)
            .is_some_and(|z| z.world_target == 0)
    }) {
        log::info!(
            "Skip respawning ResourceClump with id {} because all of its resources have a world_target of 0",
            resource_clump.id
        );
        return Ok(());
    }

    // This reducer is expected to try a single-digit or low-two-digit number of times to spawn the resource,
    // so unlike `resources_regen`, it's not beneficial to pre-compute the set of claimed tiles.
    // Instead, the `LazyMemoOccupiedTiles` queries whether a tile is claimed on the fly.
    //
    // We probably don't need to memoize at all,
    // as we're unlikely to attempt to spawn in the same tile twice within the small number of retries here,
    // but the `try_spawn_resource_multiple_attempts` algorithm
    // includes marking the location of the newly-respawned resource,
    // and I (pgoldman 2025-06-25) am too lazy to separate that logic.
    let mut occupied_tiles_hashes = LazyMemoOccupiedTiles::default();

    let chunk = ctx.db.terrain_chunk_state().chunk_index().find(timer.chunk_index).unwrap();
    if let Some(single_clump_info) = ctx.db.single_resource_clump_info().clump_id().find(timer.resource_clump_id) {
        let mut terrain_cache = TerrainChunkCache::empty();
        let mut attempts = Vec::new();

        let respawn_attempts = ctx.db.parameters_desc().version().find(0).unwrap().auto_respawn_attempts;

        // Find 10 potential coordinates within the chunk
        for _i in 0..respawn_attempts {
            // find random tile in chunk
            let min_x = chunk.chunk_x * (TerrainChunkState::WIDTH as i32) * 3;
            let min_z = chunk.chunk_z * (TerrainChunkState::HEIGHT as i32) * 3;
            let max_x = min_x + (TerrainChunkState::WIDTH as i32) * 3;
            let max_z = min_z + (TerrainChunkState::WIDTH as i32) * 3;
            let x = ctx.rng().gen_range(min_x..=max_x);
            let z = ctx.rng().gen_range(min_z..=max_z);

            let offset_coordinates = OffsetCoordinatesSmall {
                x,
                z,
                dimension: chunk.dimension,
            };
            let hex_coordinates = SmallHexTile::from(offset_coordinates);
            attempts.push(hex_coordinates);
        }
        // Final attempt will be the same location.
        attempts.push(timer.coord);

        let (spawn_result, resources_to_delete, num_attempts) = resources_regen::try_spawn_resource_multiple_attempts(
            ctx,
            &mut terrain_cache,
            &mut occupied_tiles_hashes,
            &single_clump_info.resource_clump_info,
            attempts,
        );
        if !spawn_result.is_empty() {
            log::info!(
                "Auto-respawned resource clump {} in chunk {} after {} attempts.",
                timer.resource_clump_id,
                timer.chunk_index,
                num_attempts
            );
            if !resources_to_delete.is_empty() {
                // If a resource was despawned at the same location, we need to spawn the new resource in a different transaction
                // so the client can process the OnDelete first then the OnInsert of the new one (Coord => Resource dictionary update will fail otherwise)
                for entity_id in resources_to_delete {
                    if let Some(resource) = ctx.db.resource_state().entity_id().find(entity_id) {
                        resource.despawn_self(ctx);
                    }
                }
                for res in spawn_result {
                    ResourceState::schedule_resource_spawn(ctx, res.0, res.1, res.2);
                }
            } else {
                // If no resource was despawned in order to spawn the new one, we can spawn the resource right away in the same transaction
                for res in spawn_result {
                    let health = ctx.db.resource_desc().id().find(res.0).unwrap().max_health;
                    ResourceState::spawn(ctx, None, res.0, res.1, res.2, health, false, true).unwrap();
                }
            }
            return Ok(());
        }
        log::info!(
            "Failed to auto-respawn resource clump {} in chunk {} after {} attempts.",
            timer.resource_clump_id,
            timer.chunk_index,
            num_attempts
        );
    }
    Ok(())
}
