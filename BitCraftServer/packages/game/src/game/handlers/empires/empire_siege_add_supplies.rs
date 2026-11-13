use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::game_state::{self, game_state_filters},
    messages::{components::*, empire_shared::*, game_util::ItemStack, inter_module::*},
    unwrap_or_return,
};

use super::empires_shared::validate_siege_distance;

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_add_siege_supplies(ctx: &ReducerContext, request: EmpireAddSiegeSuppliesRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;
    let player_location = game_state_filters::coordinates_any(ctx, actor_id);
    validate_siege_distance(
        ctx,
        request.building_entity_id,
        player_location,
        "You need to stand",
        2,
    )?;

    EmpireNodeSiegeState::add_supplies(ctx, actor_id, request.building_entity_id, request.proxy_empire_entity_id)
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireSiegeAddSuppliesMsg, error: Option<String>) {
    if let Some(error) = error {
        //Add supply cargo if remote call fails
        let mut player_inventory = unwrap_or_return!(
            InventoryState::get_player_inventory(ctx, request.player_entity_id),
            "Player has no inventory"
        );
        let supplies = vec![ItemStack::single_cargo(request.supply_cargo_id)];
        player_inventory.add_multiple_with_overflow(ctx, &supplies);
        ctx.db.inventory_state().entity_id().update(player_inventory);
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error, NotificationSeverity::ReducerError);
    }
}
