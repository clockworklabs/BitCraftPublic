use spacetimedb::{Identity, ReducerContext};

use crate::messages::{
    components::{user_state, NotificationSeverity, PlayerNotificationEvent},
    generic::HubItemType,
    global::{granted_hub_item_state, player_shard_state},
    inter_module::{GrantHubItemMsg, MessageContentsV5},
    static_data::premium_item_desc,
};

use super::send_inter_module_message;

pub fn send_message(
    ctx: &ReducerContext,
    player_identity: Identity,
    item_type: HubItemType,
    item_id: i32,
    quantity: u32,
    region_id: u8,
) -> Result<(), String> {
    let msg = GrantHubItemMsg {
        player_identity,
        item_type,
        item_id,
        quantity,
    };

    send_inter_module_message(
        ctx,
        MessageContentsV5::GrantHubItem(msg),
        super::InterModuleDestination::Region(region_id),
    );

    return Ok(());
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: GrantHubItemMsg, error: Option<String>) {
    if error.is_some() {
        match request.item_type {
            HubItemType::HexiteShards => {}
            HubItemType::Collectible => {
                // Reduce the granted hub items (Collectibles, Premium Items) if it was granted by the hub
                if let Some(mut granted_hub_item_state) = ctx
                    .db
                    .granted_hub_item_state()
                    .identity_and_item_id()
                    .filter((request.player_identity, request.item_id))
                    .find(|x| x.item_type == request.item_type)
                {
                    granted_hub_item_state.balance -= request.quantity;
                    ctx.db.granted_hub_item_state().entity_id().update(granted_hub_item_state);
                }
            }
            HubItemType::PremiumItem => {
                // Refund shards if the premium item was purchased from the game (and shards were deduced in-game)
                if request.item_type == HubItemType::PremiumItem {
                    let player_entity_id = ctx.db.user_state().identity().find(request.player_identity).unwrap().entity_id;
                    let premium_item_desc = ctx.db.premium_item_desc().id().find(request.item_id).unwrap();
                    let mut player_shard_state = ctx.db.player_shard_state().entity_id().find(player_entity_id).unwrap();
                    player_shard_state.shards += premium_item_desc.price;
                    ctx.db.player_shard_state().entity_id().update(player_shard_state);

                    // Send player a notificatoin (sadly this doesn't work from the global module -> todo)
                    PlayerNotificationEvent::new_event(ctx, player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
                }
            }
        }
    }
}
