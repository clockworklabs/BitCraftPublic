use bitcraft_macro::feature_gate;
use spacetimedb::{log, ReducerContext};

use crate::{
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    inter_module::*,
    messages::{components::*, empire_shared::*, game_util::ItemStack, inter_module::*, static_data::parameters_desc},
};

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_collect_hexite_capsule(ctx: &ReducerContext, request: EmpireCollectHexiteCapsuleRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let player_location = game_state_filters::coordinates_any(ctx, actor_id);
    let foundry_location = game_state_filters::coordinates(ctx, request.building_entity_id);
    if player_location.distance_to(foundry_location) > 3 {
        return Err("Too far".into());
    }

    if !EmpirePlayerDataState::has_permission_to_use_empire_building(
        ctx,
        actor_id,
        request.building_entity_id,
        EmpirePermission::CollectHexiteCapsule,
    ) {
        return Err("You don't have the permissions to collect a hexite capsule".into());
    }

    if InventoryState::get_player_cargo_id(ctx, actor_id) != 0 {
        let max_distance = ctx.db.parameters_desc().version().find(&0).unwrap().withdraw_from_deployables_range;
        let deployable_inventories =
            InventoryState::get_nearby_deployable_inventories(ctx, actor_id, |x| foundry_location.distance_to(x), max_distance);

        if deployable_inventories.len() == 0 {
            return Err("Already carrying a cargo".into());
        }

        if deployable_inventories.iter().all(|x| !x.fits(ctx, ItemStack::hexite_capsule())) {
            return Err("Not enough room in your inventory and nearby deployables".into());
        }
    }

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireCollectHexiteCapsule(EmpireCollectHexiteCapsuleMsg {
            building_entity_id: request.building_entity_id,
            player_entity_id: actor_id,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireCollectHexiteCapsuleMsg, error: Option<String>) {
    if error.is_none() {
        //Create cargo only if reducer succeeds
        let item_stack = vec![ItemStack::hexite_capsule()];
        let player_location = game_state_filters::coordinates_float(ctx, request.player_entity_id).parent_small_tile();
        let foundry_location = game_state_filters::coordinates(ctx, request.building_entity_id);

        match InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            request.player_entity_id,
            &item_stack,
            |x| foundry_location.distance_to(x),
            true,
            || vec![player_location],
            true,
        ) {
            Ok(()) => player_action_helpers::post_reducer_update_cargo(ctx, request.player_entity_id),
            Err(str) => log::error!("{str}"),
        }
    } else {
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
