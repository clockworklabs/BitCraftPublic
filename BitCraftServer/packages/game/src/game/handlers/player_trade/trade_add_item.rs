use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{entities::building_state::InventoryState, game_state},
    messages::{
        action_request::PlayerTradeAddItemRequest,
        components::*,
        game_util::{ItemType, TradePocket},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_add_item(ctx: &ReducerContext, request: PlayerTradeAddItemRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(
        ctx,
        actor_id,
        request.session_entity_id,
        request.pocket_index,
        request.inventory_pocket_index,
        request.inventory_index,
    )
}

pub fn reduce(
    ctx: &ReducerContext,
    entity_id: u64,
    session_entity_id: u64,
    pocket_index: i32,
    inventory_pocket_index: i32,
    inventory_index: i32,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    let mut trade_session = unwrap_or_err!(
        ctx.db.trade_session_state().entity_id().find(session_entity_id),
        "No such trade session."
    );

    if trade_session.status == TradeSessionStatus::SessionOffered || trade_session.status == TradeSessionStatus::SessionResolved {
        return Err("Cannot add item to this session.".into());
    }

    if entity_id != trade_session.acceptor_entity_id && entity_id != trade_session.initiator_entity_id {
        return Err("Not a member of trade session.".into());
    }

    trade_session.validate_distance(ctx)?;

    let is_initiator = entity_id == trade_session.initiator_entity_id;

    let mut player_inventory = unwrap_or_err!(
        InventoryState::get_by_owner_with_index(ctx, entity_id, inventory_index),
        "Player has no inventory"
    );

    if inventory_pocket_index as usize >= player_inventory.pockets.len() {
        return Err("Invalid inventory pocket".into());
    }

    let contents = player_inventory.get_pocket_contents(inventory_pocket_index as usize);
    if contents.is_none() {
        return Err("You can't trade an empty pocket".into());
    }
    let contents = contents.unwrap();

    let num_trade_pockets = trade_session.initiator_offer.len();

    let trade_pocket_index = if pocket_index < 0 {
        if contents.item_type == ItemType::Cargo {
            num_trade_pockets - 1
        } else {
            let position = if is_initiator {
                trade_session.initiator_offer.iter().position(|p| p.inventory_pocket_index < 0)
            } else {
                trade_session.acceptor_offer.iter().position(|p| p.inventory_pocket_index < 0)
            };
            if position.is_none() {
                return Err("No empty pockets".into());
            }
            position.unwrap()
        }
    } else {
        pocket_index as usize
    };

    if trade_pocket_index >= num_trade_pockets {
        return Err("Pocket index out of range".into());
    }

    if inventory_pocket_index < 0 {
        return Err("Invalid inventory pocket".into());
    }

    if contents.item_type == ItemType::Cargo && trade_pocket_index != num_trade_pockets - 1 {
        // Cargo is always on the last index
        return Err("You cannot add a cargo in this pocket".into());
    } else if contents.item_type == ItemType::Item && trade_pocket_index >= num_trade_pockets - 1 {
        // Other indexes carry items only
        return Err("You cannot add an item in this pocket".into());
    }

    if is_initiator {
        if trade_session.initiator_offer[trade_pocket_index].inventory_pocket_index >= 0 {
            player_inventory.unlock_pocket(inventory_pocket_index as usize);
        }

        trade_session.initiator_offer[trade_pocket_index] = TradePocket {
            inventory_index,
            inventory_pocket_index,
            contents,
        };
    } else {
        if trade_session.acceptor_offer[trade_pocket_index].inventory_pocket_index >= 0 {
            player_inventory.unlock_pocket(inventory_pocket_index as usize);
        }

        trade_session.acceptor_offer[trade_pocket_index] = TradePocket {
            inventory_index,
            inventory_pocket_index,
            contents,
        };
    }

    player_inventory.lock_pocket(inventory_pocket_index as usize);

    ctx.db.inventory_state().entity_id().update(player_inventory);

    trade_session.status = TradeSessionStatus::SessionAccepted;
    trade_session.updated_at = ctx.timestamp;

    ctx.db.trade_session_state().entity_id().update(trade_session);

    Ok(())
}
