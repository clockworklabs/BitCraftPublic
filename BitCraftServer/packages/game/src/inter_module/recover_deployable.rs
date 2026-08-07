use spacetimedb::ReducerContext;

use crate::{
    game::reducer_helpers::deployable_helpers,
    messages::{
        components::{deployable_state_v2, trade_order_state, PlayerNotificationEvent, TradeOrderState},
        inter_module::{OnDeployableRecoveredMsgV2, RecoverDeployableMsg},
        static_data::deployable_desc,
    },
    unwrap_or_err,
};

use super::send_inter_module_message;

pub fn process_message_on_destination(ctx: &ReducerContext, sender: u8, msg: RecoverDeployableMsg) -> Result<(), String> {
    let deployable = match msg.deployable_entity_id {
        0 => {
            // find deployable state to recover by player entity_id and deployable_desc_id
            ctx.db
                .deployable_state_v2()
                .owner_id()
                .filter(msg.player_entity_id)
                .find(|d| d.deployable_description_id == msg.deployable_desc_id)
        }
        _ => ctx.db.deployable_state_v2().entity_id().find(msg.deployable_entity_id),
    };

    if let Some(mut deployable) = deployable {
        if msg.deployable_desc_id != deployable.deployable_description_id {
            spacetimedb::log::error!(
                "Invalid DeployableDesc id. Received: {}, State: {}",
                msg.deployable_desc_id,
                deployable.deployable_description_id
            );
            return Err("Invalid deployable type".into());
        }
        let desc = unwrap_or_err!(
            ctx.db.deployable_desc().id().find(deployable.deployable_description_id),
            "DeployableDesc doesn't exist"
        );

        deployable_helpers::expel_and_despawn(ctx, msg.player_entity_id, deployable.entity_id, desc)?;
        ctx.db.deployable_state_v2().entity_id().delete(deployable.entity_id);
        deployable.hidden = false;
        let trade_orders: Vec<TradeOrderState> = ctx.db.trade_order_state().shop_entity_id().filter(deployable.entity_id).collect();
        ctx.db.trade_order_state().shop_entity_id().delete(deployable.entity_id);

        // if the sent deployable_entity_id was 0, we were recovering from a broken state.
        // In this case we don't send back a success message to the sender region
        if msg.deployable_entity_id != 0 {
            send_inter_module_message(
                ctx,
                crate::messages::inter_module::MessageContentsV5::OnDeployableRecovered(
                    OnDeployableRecoveredMsgV2 {
                        player_entity_id: msg.player_entity_id,
                        deployable_entity_id: deployable.entity_id,
                        deployable_desc_id: msg.deployable_desc_id,
                        deployable_state: deployable,
                        trade_orders: trade_orders,
                    },
                ),
                super::InterModuleDestination::Region(sender),
            );
        }
        return Ok(());
    }
    Err("~Deployable doesn't exist".into())
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: RecoverDeployableMsg, error: Option<String>) {
    if error.is_some() && !error.as_ref().unwrap().starts_with('~') {
        PlayerNotificationEvent::new_event(
            ctx,
            request.player_entity_id,
            error.unwrap(),
            crate::messages::components::NotificationSeverity::ReducerError,
        );
    }
}
