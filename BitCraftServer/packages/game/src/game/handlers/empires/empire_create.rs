use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::messages::static_data::*;

use crate::{
    game::{
        game_state::{self},
        reducer_helpers::{player_action_helpers::post_reducer_update_cargo, user_text_input_helpers::is_user_text_input_valid},
    },
    inter_module::*,
    messages::{
        components::*,
        empire_shared::*,
        game_util::{ItemStack, ItemType},
        inter_module::*,
        static_data::{item_desc, parameters_desc},
    },
    unwrap_or_err, unwrap_or_return,
};

#[spacetimedb::reducer]
#[feature_gate("empire")]
pub fn empire_create(ctx: &ReducerContext, request: EmpireCreateRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    UserModerationState::validate_chat_privileges(ctx, &ctx.sender, "Your naming privileges have been suspended")?;

    if let Err(_) = is_user_text_input_valid(&request.empire_name, 35, true) {
        return Err("Invalid characters in the empire name".into());
    }

    if ctx.db.empire_player_data_state().entity_id().find(&actor_id).is_some() {
        return Err("Player is already a member of an empire".into());
    }

    let claim = unwrap_or_err!(
        ctx.db.claim_state().owner_building_entity_id().find(&request.building_entity_id),
        "This is not a claim"
    );

    if claim.owner_player_entity_id != actor_id {
        return Err("Not the owner of this claim".into());
    }

    if ctx
        .db
        .empire_state()
        .capital_building_entity_id()
        .find(&request.building_entity_id)
        .is_some()
    {
        return Err("Already the capital of an empire".into());
    }

    if ctx.db.empire_state().name().find(&request.empire_name).is_some() {
        return Err("An empire with this name already exists".into());
    }

    let settlement = unwrap_or_err!(
        ctx.db
            .empire_settlement_state()
            .building_entity_id()
            .find(&request.building_entity_id),
        "This claim does not have the tech to form an empire"
    );

    if settlement.empire_entity_id != 0 {
        return Err("This claim is already part of an empire".into());
    }

    if ctx
        .db
        .empire_icon_desc()
        .id()
        .find(&request.icon_id)
        .is_none_or(|icon| icon.is_shape)
    {
        return Err("Invalid empire icon".into());
    }

    if ctx
        .db
        .empire_icon_desc()
        .id()
        .find(&request.shape_id)
        .is_none_or(|icon| !icon.is_shape)
    {
        return Err("Invalid empire shape".into());
    }

    // empire colors can only be verified on the global module?
    if ctx.db.empire_color_desc().id().find(&request.color1_id).is_none()
        || ctx.db.empire_color_desc().id().find(&request.color2_id).is_none()
    {
        return Err("Invalid empire colors".into());
    }

    let params = ctx.db.parameters_desc().version().find(&0).unwrap();

    let empire_currency_id = ctx.db.item_desc().tag().filter("Empire Currency").next().unwrap().id;

    let stack = vec![ItemStack::new(ctx, empire_currency_id, ItemType::Item, params.empire_shard_cost)];

    InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &stack, |_| 0)?;

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::EmpireCreate(EmpireCreateMsg {
            player_entity_id: actor_id,
            building_entity_id: request.building_entity_id,
            color1_id: request.color1_id,
            color2_id: request.color2_id,
            empire_name: request.empire_name,
            icon_id: request.icon_id,
            shape_id: request.shape_id,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireCreateMsg, error: Option<String>) {
    if error.is_some() {
        //Refund empire currency
        let mut player_inventory = unwrap_or_return!(
            InventoryState::get_player_inventory(ctx, request.player_entity_id),
            "Player has no inventory"
        );

        let params = ctx.db.parameters_desc().version().find(&0).unwrap();
        let empire_currency_id = ctx.db.item_desc().tag().filter("Empire Currency").next().unwrap().id;

        let supplies = vec![ItemStack::new(
            ctx,
            empire_currency_id,
            ItemType::Item,
            params.empire_shard_cost as i32,
        )];
        player_inventory.add_multiple_with_overflow(ctx, &supplies);
        ctx.db.inventory_state().entity_id().update(player_inventory);
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
    }
}
