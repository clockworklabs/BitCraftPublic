use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        discovery::Discovery,
        game_state::{self, game_state_filters},
    },
    messages::{action_request::PlayerAcquireKnowledgeFromEntitiesRequest, components::*},
    parameters_desc,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn acquire_knowledge_from_entities(ctx: &ReducerContext, request: PlayerAcquireKnowledgeFromEntitiesRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let player_coord = game_state_filters::coordinates_any(ctx, actor_id);
    let discovery_range = ctx.db.parameters_desc().version().find(0).unwrap().discovery_range;

    let mut discovery = Discovery::new(actor_id);

    for entity_id in &request.discovered_entities_id {
        // Validate that entity is within range of the player
        if let Some(location) = ctx.db.mobile_entity_state().entity_id().find(entity_id) {
            if location.coordinates().distance_to(player_coord) > discovery_range {
                // to far to discover
                continue;
            }
        }
        if let Some(location) = ctx.db.location_state().entity_id().find(entity_id) {
            if location.coordinates().distance_to(player_coord) > discovery_range {
                // to far to discover
                continue;
            }
        }

        // Learn about possible client-side entities from this point on

        // You can learn about a NPC when you open a dialogue window with them on the client
        if let Some(npc) = ctx.db.npc_state().entity_id().find(entity_id) {
            discovery.acquire_npc(ctx, npc.npc_type as i32);
        }

        // Right now there's no other case where you learn a knowledge from a client request
    }
    discovery.commit(ctx);

    Ok(())
}
