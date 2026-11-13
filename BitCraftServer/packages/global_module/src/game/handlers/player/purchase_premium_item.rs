use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    inter_module::grant_hub_item,
    messages::{
        generic::HubItemType,
        global::{player_shard_state, premium_purchase_state, user_region_state, PremiumPurchaseState},
        static_data::premium_item_desc,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn purchase_premium_item(ctx: &ReducerContext, premium_item_desc_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let user_region_state = unwrap_or_err!(ctx.db.user_region_state().identity().find(ctx.sender()), "Unknown user");
    let mut player_shard_state = unwrap_or_err!(ctx.db.player_shard_state().entity_id().find(actor_id), "Unknown PlayerShardState");
    let premium_item_desc = unwrap_or_err!(ctx.db.premium_item_desc().id().find(premium_item_desc_id), "Unknown premium item");

    if !premium_item_desc.is_enabled {
        return Err("This item is not for sale right now.".into());
    }

    if player_shard_state.shards < premium_item_desc.price {
        return Err("Not enough shards".into());
    }

    player_shard_state.shards -= premium_item_desc.price;
    ctx.db.player_shard_state().entity_id().update(player_shard_state);

    ctx.db.premium_purchase_state().insert(PremiumPurchaseState {
        entity_id: 0,
        identity: ctx.sender(),
        collectible_desc_ids: Some(premium_item_desc.collectible_ids),
        price: premium_item_desc.price,
        timestamp: ctx.timestamp,
        processed: false,
        quantity: premium_item_desc.quantity,
    });

    grant_hub_item::send_message(
        ctx,
        ctx.sender(),
        HubItemType::PremiumItem,
        premium_item_desc_id,
        premium_item_desc.quantity,
        user_region_state.region_id,
    )?;

    Ok(())
}
