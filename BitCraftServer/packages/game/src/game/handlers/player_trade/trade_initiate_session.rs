use bitcraft_macro::feature_gate;
use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::*;
use crate::{
    game::game_state::{self, game_state_filters},
    messages::{
        action_request::PlayerTradeInitiateSessionRequest,
        components::*,
        game_util::{ItemStack, TradePocket},
    },
    unwrap_or_err,
};
use crate::{i18n, parameters_desc, player_state};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn trade_initiate_session(ctx: &ReducerContext, request: PlayerTradeInitiateSessionRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    reduce(ctx, actor_id, request.acceptor_entity_id)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, acceptor_entity_id: u64) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, entity_id, true)?;

    let acceptor = unwrap_or_err!(ctx.db.player_state().entity_id().find(acceptor_entity_id), "No such player.");
    if !acceptor.signed_in {
        return Err("Cannot trade with this player.".into());
    }
    let acceptor_username = acceptor.username(ctx);

    for session in ctx.db.trade_session_state().iter() {
        if session.status == TradeSessionStatus::SessionResolved {
            continue;
        }

        if session.initiator_entity_id == entity_id || session.acceptor_entity_id == entity_id {
            return Err("Cannot start another trade".into());
        }

        if session.initiator_entity_id == acceptor_entity_id || session.acceptor_entity_id == acceptor_entity_id {
            return Err(format!("{{0}} is currently trading|~{}", i18n::dont_localize(acceptor_username)));
        }
    }

    if ThreatState::in_combat(ctx, entity_id) {
        return Err("Cannot trade while in combat".into());
    }

    if ThreatState::in_combat(ctx, acceptor_entity_id) {
        let error_string = format!("{{0}} cannot trade right now.|~{}", i18n::dont_localize(acceptor_username));
        return Err(error_string);
    }

    let max_traded_items = ctx.db.parameters_desc().version().find(0).unwrap().max_traded_items as usize;
    let trade_session_entity_id = game_state::create_entity(ctx);
    let trade_session = TradeSessionState {
        entity_id: trade_session_entity_id,
        status: TradeSessionStatus::SessionOffered,
        initiator_entity_id: entity_id,
        acceptor_entity_id,
        initiator_offer: vec![
            TradePocket {
                inventory_index: -1,
                inventory_pocket_index: -1,
                contents: ItemStack::empty(),
            };
            max_traded_items + 1    // +1 for extra cargo slot
        ],
        acceptor_offer: vec![
            TradePocket {
                inventory_index: -1,
                inventory_pocket_index: -1,
                contents: ItemStack::empty(),
            };
            max_traded_items + 1    // +1 for extra cargo slot
        ],
        updated_at: ctx.timestamp,
        resolution_message: String::new(),
    };

    let offset_coordinates = OffsetCoordinatesSmall::from(game_state_filters::coordinates_any(ctx, entity_id));

    game_state::insert_location(ctx, trade_session_entity_id, offset_coordinates);
    if ctx.db.trade_session_state().try_insert(trade_session).is_err() {
        return Err("Failed to insert trade session".into());
    }

    Ok(())
}
