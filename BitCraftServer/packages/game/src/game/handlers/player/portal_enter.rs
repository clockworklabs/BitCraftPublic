use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::*;
use crate::game::dimensions::OVERWORLD;
use crate::game::game_state::game_state_filters;
use crate::game::handlers::authentication::has_role;
use crate::game::permission_helper;
use crate::game::reducer_helpers::distance_helpers;
use crate::messages::action_request::PlayerPortalEnterRequest;
use crate::messages::components::*;
use crate::messages::events::*;
use crate::messages::static_data::building_portal_desc;
use crate::{deployable_desc, game_state, ClaimPermission};
use crate::{parameters_desc, unwrap_or_err};

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn portal_enter(ctx: &ReducerContext, request: PlayerPortalEnterRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let portal = unwrap_or_err!(ctx.db.portal_state().entity_id().find(request.portal_entity_id), "Invalid portal");

    if !portal.enabled {
        return Err("Portal is not enabled".into());
    }

    let player_position = game_state_filters::coordinates_any(ctx, actor_id);
    let building_position = game_state::game_state_filters::coordinates(ctx, request.portal_entity_id);

    if player_position.dimension == OVERWORLD {
        if !permission_helper::can_interact_with_tile(ctx, building_position, actor_id, ClaimPermission::Usage) {
            // Make sure there is no explicit claim permission override for the building before denying access
            let portal_building = ctx.db.building_state().entity_id().find(request.portal_entity_id).unwrap();
            if !permission_helper::can_interact_with_building(ctx, &portal_building, actor_id, ClaimPermission::Usage) {
                return Err("You don't have permission to enter this building".into());
            }
        }
    } else {
        // Interior specific checks-

        // Check if all enemies are dead before allowing to enter the portal
        let portal_building_entity_id = ctx
            .db
            .building_state()
            .entity_id()
            .find(request.portal_entity_id)
            .unwrap()
            .building_description_id;
        let building_portal_desc = ctx
            .db
            .building_portal_desc()
            .building_id()
            .filter(portal_building_entity_id)
            .next()
            .unwrap();

        if building_portal_desc.enemy_lock
            && ctx
                .db
                .mobile_entity_state()
                .dimension()
                .filter(player_position.dimension)
                .any(|mes| {
                    ctx.db.enemy_state().entity_id().find(mes.entity_id).is_some()
                }
                && !has_role(ctx, &ctx.sender(), crate::messages::authentication::Role::Gm))
            {
                return Err("You must defeat all enemies in this section before you can proceed".into());
            }
    }

    let mut is_mounting_deployable = false;

    if let Some(mounting) = ctx.db.mounting_state().entity_id().find(actor_id) {
        let deployable = ctx.db.deployable_state_v2().entity_id().find(mounting.deployable_entity_id).unwrap();
        let deployable_desc = ctx.db.deployable_desc().id().find(deployable.deployable_description_id).unwrap();
        if !deployable_desc.can_enter_portals {
            return Err("This deployable cannot enter portals".into());
        }
        is_mounting_deployable = true;
    }

    let distance = distance_helpers::distance_to_portal(ctx, request.portal_entity_id, player_position, is_mounting_deployable)?;
    if distance > 2 {
        return Err("You are too far from this portal".into());
    }

    // Refresh Innerlight when the portal destination is overworld
    if portal.destination_dimension == OVERWORLD {
        let mut active_buff_state = unwrap_or_err!(
            ctx.db.active_buff_state().entity_id().find(actor_id),
            "Player has no active buff state."
        );

        let innerlight_buff_duration = ctx.db.parameters_desc().version().find(0).unwrap().sign_in_aggro_immunity; //note: separate parameter?
        active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);
        ctx.db.active_buff_state().entity_id().update(active_buff_state);
    }

    let oc = OffsetCoordinatesSmall {
        x: portal.destination_x,
        z: portal.destination_z,
        dimension: portal.destination_dimension,
    };
    let ofc = OffsetCoordinatesFloat::from(oc);
    game_state_filters::teleport_to(ctx, actor_id, ofc, false, 0.0)?;
    ctx.db.player_teleport_event().insert(PlayerTeleportEvent { actor_id });
    Ok(())
}
