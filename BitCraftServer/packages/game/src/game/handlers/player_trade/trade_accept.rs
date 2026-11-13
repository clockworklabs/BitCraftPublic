use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{
        discovery::Discovery, entities::building_state::InventoryState, game_state,
        reducer_helpers::player_action_helpers::post_reducer_update_cargo,
    },
    messages::{action_request::PlayerTradeAcceptRequest, components::*, game_util::ItemStack},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_accept(ctx: &ReducerContext, request: PlayerTradeAcceptRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.session_entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, session_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    let mut trade_session = unwrap_or_err!(
        ctx.db.trade_session_state().entity_id().find(session_entity_id),
        "No such trade session."
    );

    if trade_session.status == TradeSessionStatus::SessionOffered || trade_session.status == TradeSessionStatus::SessionResolved {
        return Err("Cannot accept this trade.".into());
    }

    if entity_id != trade_session.acceptor_entity_id && entity_id != trade_session.initiator_entity_id {
        return Err("Not a member of trade session.".into());
    }

    trade_session.validate_distance(ctx)?;

    if entity_id == trade_session.initiator_entity_id {
        if trade_session.status == TradeSessionStatus::AcceptorAccepted {
            // If the acceptor accepted and the initiator is accepting finalize the trade.
            finalize(ctx, &mut trade_session)?;
        } else if trade_session.status == TradeSessionStatus::SessionAccepted {
            // If no one has accepted then have the initiator accept
            trade_session.status = TradeSessionStatus::InitiatorAccepted;
        } else {
            // If the initiator has accepted then toggle the initiator to not accept
            // TODO: Should unaccept be it's own event?
            trade_session.status = TradeSessionStatus::SessionAccepted;
        }
    } else {
        if trade_session.status == TradeSessionStatus::InitiatorAccepted {
            // Inverse of above
            finalize(ctx, &mut trade_session)?;
        } else if trade_session.status == TradeSessionStatus::SessionAccepted {
            // If no one has accepted then have the acceptor accept
            trade_session.status = TradeSessionStatus::AcceptorAccepted;
        } else {
            // If the acceptor has accepted then toggle the acceptor to not accept
            // TODO: Should unaccept be it's own event?
            trade_session.status = TradeSessionStatus::SessionAccepted;
        }
    }

    trade_session.updated_at = ctx.timestamp;

    post_reducer_update_cargo(ctx, trade_session.acceptor_entity_id);
    post_reducer_update_cargo(ctx, trade_session.initiator_entity_id);

    ctx.db.trade_session_state().entity_id().update(trade_session);

    Ok(())
}

pub fn finalize(ctx: &ReducerContext, trade_session: &mut TradeSessionState) -> Result<(), String> {
    trade_session.status = TradeSessionStatus::SessionResolved;

    let initiator_offer = trade_session.initiator_offer.clone();

    let mut inventories_updated: Vec<InventoryState> = vec![];

    let mut initiator_offered: Vec<ItemStack> = vec![];
    for trade_pocket in initiator_offer.into_iter() {
        let inventory_pocket_index = trade_pocket.inventory_pocket_index;
        let inventory_index = trade_pocket.inventory_index;
        if inventory_pocket_index < 0 || inventory_index < 0 {
            continue;
        }

        let mut initiator_inventory = &mut unwrap_or_err!(
            InventoryState::get_by_owner_with_index(ctx, trade_session.initiator_entity_id, inventory_index),
            "Initiator has no inventory"
        );

        let inventory_entity_id = initiator_inventory.entity_id;
        let inventory_index: usize;
        if let Some(i) = inventories_updated.iter().position(|inv| inv.entity_id == inventory_entity_id) {
            inventory_index = i;
        } else {
            inventories_updated.push(initiator_inventory.to_owned());
            inventory_index = inventories_updated.len() - 1;
        }

        initiator_inventory = &mut inventories_updated[inventory_index];

        let inventory_pocket_index = inventory_pocket_index as usize;

        if let Some(content) = initiator_inventory.get_pocket_contents(inventory_pocket_index) {
            if content != trade_pocket.contents {
                return Err("Initiator missing required items.".into());
            }
        } else {
            return Err("Initiator missing required items.".into());
        }

        let contents = initiator_inventory.remove_at(inventory_pocket_index).unwrap();
        initiator_offered.push(contents);
    }

    let acceptor_offer = trade_session.acceptor_offer.clone();

    let mut acceptor_offered: Vec<ItemStack> = vec![];
    for trade_pocket in acceptor_offer.into_iter() {
        let inventory_pocket_index = trade_pocket.inventory_pocket_index;
        let inventory_index = trade_pocket.inventory_index;
        if inventory_pocket_index < 0 || inventory_index < 0 {
            continue;
        }

        //use for entity id
        let mut acceptor_inventory = &mut unwrap_or_err!(
            InventoryState::get_by_owner_with_index(ctx, trade_session.acceptor_entity_id, inventory_index),
            "Acceptor has no inventory"
        );

        let inventory_entity_id = acceptor_inventory.entity_id;
        let inventory_index: usize;
        if let Some(i) = inventories_updated.iter().position(|inv| inv.entity_id == inventory_entity_id) {
            inventory_index = i;
        } else {
            inventories_updated.push(acceptor_inventory.to_owned());
            inventory_index = inventories_updated.len() - 1;
        }

        acceptor_inventory = &mut inventories_updated[inventory_index];

        let inventory_pocket_index = inventory_pocket_index as usize;

        if let Some(content) = acceptor_inventory.get_pocket_contents(inventory_pocket_index) {
            if content != trade_pocket.contents {
                return Err("Acceptor missing required items.".into());
            }
        } else {
            return Err("Acceptor missing required items.".into());
        }

        let contents = acceptor_inventory.remove_at(inventory_pocket_index).unwrap();
        acceptor_offered.push(contents);
    }

    for inventory_updated in inventories_updated {
        let mut updated_inventory = inventory_updated.to_owned();
        updated_inventory.unlock_all_pockets();
        ctx.db.inventory_state().entity_id().update(updated_inventory);
    }

    let mut initiator_discovery = Discovery::new(trade_session.initiator_entity_id);
    let mut acceptor_discovery = Discovery::new(trade_session.acceptor_entity_id);

    for item_stack in initiator_offered {
        if !InventoryState::add_and_discover(ctx, trade_session.acceptor_entity_id, &mut acceptor_discovery, item_stack, false) {
            return Err("Acceptor inventory too full to accept.".into());
        }
    }

    for item_stack in acceptor_offered {
        if !InventoryState::add_and_discover(ctx, trade_session.initiator_entity_id, &mut initiator_discovery, item_stack, false) {
            return Err("Initiator inventory too full to accept.".into());
        }
    }

    acceptor_discovery.commit(ctx);
    initiator_discovery.commit(ctx);

    Ok(())
}
