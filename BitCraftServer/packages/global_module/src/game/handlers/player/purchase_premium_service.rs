use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, handlers::player::player_set_name},
    messages::{
        components::player_username_state,
        generic::PremiumServiceType,
        global::{player_shard_state, premium_purchase_state, PremiumPurchaseState},
        static_data::{premium_service_desc, PremiumServiceDesc},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn purchase_character_rename(ctx: &ReducerContext, premium_service_desc_id: i32, new_character_name: String) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    let player_username_state = unwrap_or_err!(ctx.db.player_username_state().entity_id().find(actor_id), "Unknown player");
    if player_username_state.username == new_character_name {
        return Err("This username is taken.".into());
    }

    let premium_service_desc: PremiumServiceDesc = unwrap_or_err!(
        ctx.db.premium_service_desc().id().find(premium_service_desc_id),
        "Unknown premium service"
    );

    if !premium_service_desc.is_enabled {
        return Err("This service is not for sale right now.".into());
    }

    if premium_service_desc.service_type != PremiumServiceType::CharacterRename {
        return Err("Invalid premium service".into());
    }

    if player_username_state.username != format!("player{}", actor_id) {
        purchase_premium_service(ctx, actor_id, &premium_service_desc, false)?;
    }
    player_set_name::reduce(ctx, actor_id, new_character_name)
}

fn purchase_premium_service(
    ctx: &ReducerContext,
    actor_id: u64,
    premium_service_desc: &PremiumServiceDesc,
    deduct_hub_shards: bool,
) -> Result<(), String> {
    let mut player_shard_state = unwrap_or_err!(ctx.db.player_shard_state().entity_id().find(actor_id), "Unknown PlayerShardState");

    if player_shard_state.shards < premium_service_desc.price {
        return Err("Not enough shards".into());
    }

    if deduct_hub_shards {
        ctx.db.premium_purchase_state().insert(PremiumPurchaseState {
            entity_id: 0,
            identity: ctx.sender(),
            collectible_desc_ids: None,
            price: premium_service_desc.price,
            timestamp: ctx.timestamp,
            processed: false,
            quantity: 1,
        });
    }

    player_shard_state.shards -= premium_service_desc.price;
    ctx.db.player_shard_state().entity_id().update(player_shard_state);

    Ok(())
}
