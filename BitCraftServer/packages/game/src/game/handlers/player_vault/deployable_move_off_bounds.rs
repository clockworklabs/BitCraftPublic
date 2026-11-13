use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, reducer_helpers::deployable_helpers::expel_passengers, terrain_chunk::TerrainChunkCache},
    messages::{
        components::*,
        static_data::{deployable_desc, DeployableType, FootprintType},
    },
    unwrap_or_err, OffsetCoordinatesFloat, SmallHexTile,
};
use crate::messages::events::*;

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_move_off_bounds(ctx: &ReducerContext, deployable_entity_id: u64) -> Result<(), String> {
    // Note:
    // This code is pretty much identical to the deployable_move_off_claims code, except for ownership check and that it looks for footprints

    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    move_deployable_off_bounds(ctx, deployable_entity_id, Some(actor_id))
}

// Server-initiated relocation has no player actor to authorize, but otherwise uses
// the same search and state updates as the player-facing recovery reducer.
pub fn move_deployable_off_bounds(ctx: &ReducerContext, deployable_entity_id: u64, actor_id: Option<u64>) -> Result<(), String> {
    let location = unwrap_or_err!(
        ctx.db.mobile_entity_state().entity_id().find(deployable_entity_id),
        "Deployable does not exist"
    );

    let center = location.coordinates();

    let mut deployable = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id),
        "Deployable does not exist"
    );

    if !FootprintTileState::get_at_location(ctx, &center).any(|f| f.footprint_type == FootprintType::Hitbox) {
        return Err("Deployable is not stuck".into());
    }

    // Ignore siege engines requirements for now (there's no harm if anyone does that, really) - client check is enough for that.

    let deployable_desc = ctx.db.deployable_desc().id().find(deployable.deployable_description_id).unwrap();
    if let Some(actor_id) = actor_id {
        if deployable_desc.deployable_type != DeployableType::SiegeEngine && deployable.owner_id != actor_id {
            return Err("Only the deployable owner can move it out of bounds".into());
        }
    }

    let mut radius = 1;
    let mut terrain_cache = TerrainChunkCache::empty();
    let mut tentative_tile = None;
    let mut max_radius = 20;

    while radius < max_radius {
        for coord in SmallHexTile::ring_iter(center, radius) {
            if FootprintTileState::get_at_location(ctx, &coord).any(|f| f.footprint_type == FootprintType::Hitbox) {
                continue;
            }

            let large_tile = coord.parent_large_tile();
            if let Some(terrain_tile) = terrain_cache.get_terrain_cell(ctx, &large_tile) {
                if terrain_tile.is_submerged() {
                    if tentative_tile.is_none() {
                        // Once we find a water tile, still look for a ground tile within 6 loops.
                        tentative_tile = Some(coord);
                        max_radius = radius + 6;
                    }
                } else {
                    // we found a ground tile, let's end the search immediately.
                    tentative_tile = Some(coord);
                    max_radius = -1;
                    break;
                }
            }
        }
        radius += 1;
    }

    let tile = unwrap_or_err!(tentative_tile, "Unable to find a valid tile to move the deployable");

    // Dismount anyone in the deployable that gets moved off
    expel_passengers(ctx, deployable_entity_id, true, true);

    let offset = tile.to_offset_coordinates();
    let offset_float = OffsetCoordinatesFloat::from(offset);
    let new_location = MobileEntityState::for_location(
        deployable_entity_id,
        (offset_float.x, offset_float.z, offset_float.dimension).into(),
        ctx.timestamp,
    );

    // update icon on map
    let mut deployable_collectible = ctx
        .db
        .deployable_collectible_state()
        .deployable_entity_id()
        .find(deployable.entity_id)
        .unwrap();
    deployable_collectible.location = Some(offset);
    ctx.db
        .deployable_collectible_state()
        .deployable_entity_id()
        .update(deployable_collectible);

    if deployable.claim_entity_id != 0 {
        deployable.claim_entity_id = 0;
        ctx.db.deployable_state_v2().entity_id().update(deployable);
    }

    ctx.db.mobile_entity_state().entity_id().update(new_location);
    if let Some(actor_id) = actor_id {
        ctx.db.deployable_disembark_event().insert(DeployableDisembarkEvent {
            actor_id,
            deployable_entity_id,
        });
    }

    Ok(())
}
