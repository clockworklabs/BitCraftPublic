use spacetimedb::{log, ReducerContext};

use crate::{
    game::game_state::{self, game_state_filters},
    inter_module::*,
    messages::{
        components::*,
        empire_shared::{EmpirePermission, EmpirePlayerDataState},
        game_util::{ItemStack, ItemType},
        inter_module::*,
        static_data::item_desc,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn empire_withdraw_item(ctx: &ReducerContext, item_id: i32, item_type: ItemType, amount: u32) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::WithdrawEmpireCurrency) {
        return Err("You don't have the permission to withdraw from the Empire treasury".into());
    }

    // No need to check inventory room for now since we can only withdraw wallet items at the moment

    if item_type == ItemType::Item {
        let item_desc = unwrap_or_err!(ctx.db.item_desc().id().find(item_id), "Invalid item");
        match item_desc.tag.as_str() {
            "Empire Currency" => {}
            _ => return Err("This item can't be withdrawn".into()),
        }
    } else {
        // for now no cargo can be donated
        return Err("This cargo can't be withdrawn".into());
    }

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireWithdrawItem(EmpireWithdrawItemMsg {
            player_entity_id: actor_id,
            item_id: item_id,
            count: amount,
            is_cargo: item_type == ItemType::Cargo,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireWithdrawItemMsg, error: Option<String>) {
    if error.is_some() {
        // Nothing was withdrawn
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    } else {
        let items = vec![ItemStack {
            item_id: request.item_id,
            quantity: request.count as i32,
            item_type: if request.is_cargo { ItemType::Cargo } else { ItemType::Item },
            durability: None,
        }];

        // TODO -> validate inventory FIRST if this can fail. Right now it can't as we're withdrawing wallet items.
        match InventoryState::deposit_to_player_inventory_and_nearby_deployables(
            ctx,
            request.player_entity_id,
            &items,
            |_x| 0,
            false,
            || vec![{ game_state_filters::coordinates_any(ctx, request.player_entity_id) }],
            true,
        ) {
            Ok(()) => {}
            Err(str) => log::error!("{str}"),
        }
    }
}
