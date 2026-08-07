use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        game_state::{self, game_state_filters},
        handlers::inventory::inventory_helper,
        reducer_helpers::player_action_helpers::post_reducer_update_cargo,
    },
    inter_module::*,
    messages::{
        components::*,
        empire_shared::*,
        game_util::{ItemStack, ItemType},
        inter_module::*,
        static_data::item_desc,
    },
    unwrap_or_err, unwrap_or_return,
};

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_donate_item(ctx: &ReducerContext, request: EmpireDonateItemRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    let from_pocket = request.from_pocket;

    // Check if the player still carries the item
    let mut inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(from_pocket.inventory_entity_id),
        "Missing inventory"
    );

    let player_location = game_state_filters::coordinates_any(ctx, actor_id);

    inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        inventory.owner_entity_id,
        inventory.player_owner_entity_id,
    )?;

    if from_pocket.pocket_index < 0 || from_pocket.pocket_index >= inventory.pockets.len() as i32 {
        return Err("Invalid pocket index".into());
    }
    let item_stack = unwrap_or_err!(inventory.get_at(from_pocket.pocket_index as usize), "Pocket is empty");

    if item_stack.quantity <= 0 {
        return Err("Pocket is empty".into());
    }

    if item_stack.item_type == ItemType::Item {
        let item_desc = unwrap_or_err!(ctx.db.item_desc().id().find(item_stack.item_id), "Invalid item");
        match item_desc.tag.as_str() {
            "Empire Currency" => {}
            _ => return Err("This item can't be donated".into()),
        }
    } else {
        // for now no cargo can be donated
        return Err("This cargo can't be donated".into());
    }

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireDonateItem(EmpireDonateItemMsg {
            player_entity_id: actor_id,
            item_id: item_stack.item_id,
            count: item_stack.quantity as u32,
            is_cargo: item_stack.item_type == ItemType::Cargo,
            on_behalf_username: None,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    inventory.set_at(from_pocket.pocket_index as usize, None);
    ctx.db.inventory_state().entity_id().update(inventory);

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireDonateItemMsg, error: Option<String>) {
    if error.is_some() {
        //Add supply cargo if remote call fails
        let mut player_inventory = unwrap_or_return!(
            InventoryState::get_player_inventory(ctx, request.player_entity_id),
            "Player has no inventory"
        );
        let supplies = vec![ItemStack::new(
            ctx,
            request.item_id,
            if request.is_cargo { ItemType::Cargo } else { ItemType::Item },
            request.count as i32,
        )];
        player_inventory.add_multiple_with_overflow(ctx, &supplies);
        ctx.db.inventory_state().entity_id().update(player_inventory);
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
