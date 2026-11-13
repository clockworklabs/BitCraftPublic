use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::game_state::{self, game_state_filters};
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::game::PLAYER_MIN_SWIM_DEPTH;
use crate::messages::action_request::PlayerDeployableMountRequest;
use crate::messages::components::*;
use crate::messages::events::*;
use crate::{deployable_desc, unwrap_or_err, MovementType};

#[spacetimedb::reducer]
#[feature_gate]
pub fn deployable_mount(ctx: &ReducerContext, request: PlayerDeployableMountRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let deployable_entity_id = request.deployable_entity_id;

    if ctx.db.mounting_state().entity_id().find(actor_id).is_some() {
        return Err("Already mounting a deployable".into());
    }

    let deployable = unwrap_or_err!(
        ctx.db.deployable_state_v2().entity_id().find(deployable_entity_id),
        "Deployable not found!"
    );
    let deployable_desc = unwrap_or_err!(
        ctx.db.deployable_desc().id().find(deployable.deployable_description_id),
        "Invalid deployable type"
    );

    if deployable_desc.capacity == 0 {
        return Err("You can't board this".into());
    }

    let player_coordinates = game_state_filters::coordinates_float(ctx, actor_id);
    let deployable_coordinates = game_state_filters::coordinates_float(ctx, deployable_entity_id);

    if player_coordinates.distance_to(deployable_coordinates) > deployable_desc.mounting_radius {
        return Err("Too far".into());
    }

    // Prevent boarding a deployable that is at an illegal location
    if deployable_desc.movement_type == MovementType::Water {
        let mut terrain_cache = TerrainChunkCache::empty();
        if let Some(terrain) = terrain_cache.get_terrain_cell(ctx, &deployable_coordinates.into()) {
            if !terrain.is_submerged() {
                return Err("You can't board this deployable while it's grounded".into());
            }
        }
    } else if deployable_desc.movement_type == MovementType::Ground {
        let mut terrain_cache = TerrainChunkCache::empty();
        if let Some(terrain) = terrain_cache.get_terrain_cell(ctx, &deployable_coordinates.into()) {
            if terrain.water_depth() >= PLAYER_MIN_SWIM_DEPTH {
                return Err("You can't board this deployable while it's submerged".into());
            }
        }
    }

    let is_owner = deployable.owner_id == actor_id;

    let mut selected_slot = 0;
    // Owner is always the driver
    if !is_owner {
        // Non owner will take a passenger seat if available. Never the driver slot in this version.
        let free_slots = DeployableStateV2::free_slots(ctx, deployable_entity_id);
        if let Some(passenger_slot) = free_slots.iter().find(|s| **s != 0) {
            selected_slot = *passenger_slot;
        } else {
            return Err("No passenger seat available".into());
        }
    }

    // Explore deployable tile
    PlayerState::move_player_and_explore(
        ctx,
        actor_id,
        &player_coordinates,
        &deployable_coordinates,
        0.0,
        false,
        None,
    )?;

    let mounting_state = MountingState {
        entity_id: actor_id,
        deployable_entity_id,
        deployable_slot: selected_slot,
    };

    // Create mounting component for player
    if ctx.db.mounting_state().try_insert(mounting_state).is_err() {
        return Err("Failed to insert mounting state".into());
    }

    PlayerState::collect_stats(ctx, actor_id);
    ctx.db.deployable_mount_event().insert(DeployableMountEvent {
        actor_id,
        deployable_entity_id,
    });

    Ok(())
}
