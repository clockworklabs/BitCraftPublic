use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{coordinates::OffsetCoordinatesSmall, reducer_helpers::interior_helpers},
    messages::{
        action_request::ServerTeleportReason,
        components::*,
        inter_module::{MessageContentsV5, TransferPlayerHousingMsg},
        static_data::{collectible_desc, CollectibleType},
    },
    unwrap_or_err,
};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, player_housing_entity_id: u64, new_entrance_building_entity_id: u64) -> Result<(), String> {
    //Add client event
    if let Some(player_housing) = ctx.db.player_housing_state().entity_id().find(player_housing_entity_id) {
        send_inter_module_message(
            ctx,
            MessageContentsV5::TransferPlayerHousingRequest(TransferPlayerHousingMsg {
                player_entity_id: player_housing_entity_id,
                new_entrance_building_entity_id,
                network_entity_id: player_housing.network_entity_id,
                interior_portal_entity_id: player_housing.exit_portal_entity_id,
            }),
            super::InterModuleDestination::Region(player_housing.region_index),
        );
    }
    return Ok(());
}

pub fn process_message_on_destination(ctx: &ReducerContext, msg: TransferPlayerHousingMsg) -> Result<(), String> {
    // Player housing no longer exists in this region, but the dimension_network_state does.
    if !PlayerHousingState::is_dimension_network_empty(ctx, msg.network_entity_id) {
        return Err(
            "Player housing needs to be empty to be transferred across regions. Some other player might have modified the interior.".into(),
        );
    }

    let player_housing = unwrap_or_err!(
        ctx.db.player_housing_state().network_entity_id().find(msg.network_entity_id),
        "Housing doesn't exist"
    );
    player_housing.expel_players_and_entities(ctx, ServerTeleportReason::PlayerHousingChangedLocation);

    let portal = ctx.db.portal_state().entity_id().find(msg.interior_portal_entity_id).unwrap();

    let oc = OffsetCoordinatesSmall {
        x: portal.destination_x,
        z: portal.destination_z,
        dimension: portal.destination_dimension,
    };

    // delete player housing cost
    ctx.db.player_housing_moving_cost_state().entity_id().delete(msg.player_entity_id);
    ctx.db.player_housing_customization_state().entity_id().delete(msg.player_entity_id);

    // Destroy dimension network
    interior_helpers::delete_dimension_network(ctx, msg.network_entity_id, oc.into());
    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: TransferPlayerHousingMsg, error: Option<String>) {
    let actor_id = request.player_entity_id;

    if let Some(error_msg) = error {
        // do not transfer housing, something happened.
        PlayerNotificationEvent::new_event(ctx, actor_id, error_msg, NotificationSeverity::ActionDenied);
        return;
    }

    // delete empty player housing, we will re-create it with the proper dimensions and everything.
    if let Some(housing) = ctx.db.player_housing_state().entity_id().find(request.player_entity_id) {
        PlayerHousingState::delete_shared(ctx, housing, super::InterModuleDestination::GlobalAndAllOtherRegions);
    }

    // create player housing cost
    ctx.db.player_housing_moving_cost_state().insert(PlayerHousingMovingCostState {
        entity_id: actor_id,
        moving_time_cost_minutes: 0,
    });

    // transfer housing here.
    match PlayerHousingState::get_and_validate_entrance_building(ctx, actor_id, request.new_entrance_building_entity_id) {
        Ok(building) => {
            let outside_dimension = ctx
                .db
                .location_state()
                .entity_id()
                .find(request.new_entrance_building_entity_id)
                .unwrap()
                .dimension;

            let (highest_rank, template_building_id) = PlayerHousingState::get_rank_and_template_building(ctx, actor_id);
            match PlayerHousingState::create_housing(ctx, actor_id, highest_rank, template_building_id, &building, outside_dimension) {
                Err(error_msg) => PlayerNotificationEvent::new_event(ctx, actor_id, error_msg, NotificationSeverity::ActionDenied),
                Ok(()) => {
                    let vault = ctx.db.vault_state().entity_id().find(request.player_entity_id).unwrap();
                    let wall_collectible_id = match vault.collectibles.iter().find(|c| {
                        c.activated && ctx.db.collectible_desc().id().find(c.id).unwrap().collectible_type == CollectibleType::HousingWalls
                    }) {
                        Some(c) => c.id,
                        None => 0,
                    };
                    let floor_collectible_id = match vault.collectibles.iter().find(|c| {
                        c.activated && ctx.db.collectible_desc().id().find(c.id).unwrap().collectible_type == CollectibleType::HousingFloor
                    }) {
                        Some(c) => c.id,
                        None => 0,
                    };
                    // Re-apply customization
                    ctx.db
                        .player_housing_customization_state()
                        .entity_id()
                        .update(PlayerHousingCustomizationState {
                            entity_id: actor_id,
                            wall_collectible_id,
                            floor_collectible_id,
                        });
                }
            };
            return;
        }
        Err(error_msg) => PlayerNotificationEvent::new_event(ctx, actor_id, error_msg, NotificationSeverity::ActionDenied),
    }
}
