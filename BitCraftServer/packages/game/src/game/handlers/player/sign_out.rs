use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, Identity, ReducerContext, Table};

use crate::deployable_desc;
use crate::game::handlers::player_vault::deployable_hide::{hide_deployable_timer, HideDeployableTimer};
use crate::game::handlers::queue::end_grace_period::{EndGracePeriodTimer, GracePeriodType};
use crate::game::handlers::server::player_clear_action_state;
use crate::messages::queue::player_queue_state;
use crate::messages::region::{player_session_log, PlayerSessionLog};
use crate::{
    game::{
        game_state::{self, game_state_filters},
        reducer_helpers::{restore_player_helpers::auto_unstuck_player_and_deployable, timer_helpers::now_plus_secs},
    },
    messages::components::*,
};

// this was a reducer in Bitcraft but here we just call it directly
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn sign_out(ctx: &ReducerContext) {
    let now_ms = ctx.timestamp;
    log::info!("[{:?}] [{:?}] sign_out", now_ms, &ctx.sender().to_hex());

    sign_out_internal(ctx, ctx.sender(), true);
}

pub fn sign_out_internal(ctx: &ReducerContext, identity: Identity, insert_grace_period: bool) {
    if let Some(mut user) = ctx.db.user_state().identity().find(&identity) {
        PlayerTimestampState::clear(ctx, user.entity_id);

        let player_entity_id = user.entity_id;

        if let Some(mut player) = ctx.db.player_state().entity_id().find(&player_entity_id) {
            if !player.signed_in {
                log::info!("(sign_out) Already signed Out for {:?}", identity.to_hex());
                return;
            }

            log::info!("Signout starting for {:?}", identity.to_hex());

            // Note: we used to disembark from a land deployable when logging out, but that could be exploited and used to disembark over normally blocked spots.
            // The new way the deployables work (vault items that you store/deploy) mitigate the need to expulse unwanted logged-off passengers.

            if insert_grace_period {
                if user.can_sign_in {
                    //When this expires, it will processes the queue and set can_sign_in back to false
                    EndGracePeriodTimer::new(ctx, identity, GracePeriodType::SignIn);
                } else if ctx.db.player_queue_state().entity_id().find(user.entity_id).is_some() {
                    //When this expires, it will dequeue the player
                    EndGracePeriodTimer::new(ctx, identity, GracePeriodType::QueueJoin);
                }
            } else {
                user.can_sign_in = false;
                ctx.db.user_state().entity_id().update(user);
            }

            player.signed_in = false;

            if player.session_start_timestamp != 0 {
                // in case we log off while minimized
                let time_played = game_state::unix(ctx.timestamp) - player.session_start_timestamp;
                player.time_played += time_played;
                player.session_start_timestamp = 0;
            }
            if player.sign_in_timestamp != 0 {
                ctx.db.player_session_log().insert(PlayerSessionLog {
                    id: 0,
                    entity_id: player_entity_id,
                    sign_in_timestamp: player.sign_in_timestamp,
                    sign_out_timestamp: game_state::unix(ctx.timestamp),
                });
            }
            player.time_signed_in += game_state::unix(ctx.timestamp) - player.sign_in_timestamp;
            player.sign_in_timestamp = 0;

            ctx.db.player_state().entity_id().update(player);

            // restart buff timestamp
            if let Some(mut active_buff_state) = ctx.db.active_buff_state().entity_id().find(&player_entity_id) {
                active_buff_state.pause_all_buffs(ctx);
                ctx.db.active_buff_state().entity_id().update(active_buff_state);
            } else {
                log::error!("Player {player_entity_id} has no ActiveBuffState");
            }

            // clear current player target
            ctx.db.target_state().entity_id().delete(&player_entity_id);

            // end all combat sessions involving this player
            ThreatState::clear_all(ctx, player_entity_id);

            // clear anyone targetting the dead player
            game_state_filters::untarget(ctx, player_entity_id);

            auto_unstuck_player_and_deployable(ctx, &player_entity_id);

            // interrupt player and deployable movement
            if let Some(mut pos) = ctx.db.mobile_entity_state().entity_id().find(&player_entity_id) {
                if let Some(network) = ctx.db.dimension_description_state().dimension_id().find(pos.dimension) {
                    InteriorPlayerCountState::dec(ctx, network.dimension_network_entity_id);
                }

                if pos.destination_x != pos.location_x || pos.destination_z != pos.location_z {
                    pos.destination_x = pos.location_x;
                    pos.destination_z = pos.location_z;
                    pos.timestamp = game_state::unix_ms(ctx.timestamp);
                    ctx.db.mobile_entity_state().entity_id().update(pos);
                }
            } else {
                log::error!("Player {player_entity_id} has no MobileEntityState");
            }
            if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&player_entity_id) {
                if let Some(mut pos) = ctx.db.mobile_entity_state().entity_id().find(&mounting.deployable_entity_id) {
                    if pos.destination_x != pos.location_x || pos.destination_z != pos.location_z {
                        pos.destination_x = pos.location_x;
                        pos.destination_z = pos.location_z;
                        pos.timestamp = game_state::unix_ms(ctx.timestamp);
                        ctx.db.mobile_entity_state().entity_id().update(pos);
                    }
                } else {
                    log::error!("Player {player_entity_id} has MountingState without MobileEntityState");
                }
            }

            //start hide deployable timer
            for deployable in ctx.db.deployable_state_v2().owner_id().filter(player_entity_id) {
                let deployable_desc = ctx
                    .db
                    .deployable_desc()
                    .id()
                    .find(&deployable.deployable_description_id)
                    .unwrap();
                if deployable_desc.show_for_secs_after_owner_logout >= 0 {
                    // Only hide deployed deployables
                    if ctx.db.mobile_entity_state().entity_id().find(&deployable.entity_id).is_some() {
                        let scheduled_at =
                            if deployable_desc.barter > 0 && !TradeOrderState::has_any_in_stock_trade_orders(ctx, deployable.entity_id) {
                                now_plus_secs(0, ctx.timestamp)
                            } else {
                                now_plus_secs(deployable_desc.show_for_secs_after_owner_logout as u64, ctx.timestamp)
                            };

                        ctx.db.hide_deployable_timer().entity_id().delete(deployable.entity_id);
                        ctx.db
                            .hide_deployable_timer()
                            .try_insert(HideDeployableTimer {
                                scheduled_id: 0,
                                scheduled_at: scheduled_at,
                                entity_id: deployable.entity_id,
                            })
                            .ok()
                            .unwrap();
                    }
                }
            }

            //cancel trade sessions
            for trade in ctx.db.trade_session_state().initiator_entity_id().filter(player_entity_id) {
                cancel_trade(ctx, trade);
            }
            for trade in ctx.db.trade_session_state().acceptor_entity_id().filter(player_entity_id) {
                cancel_trade(ctx, trade);
            }

            //Cancel actions
            _ = player_clear_action_state::reduce(
                ctx,
                player_entity_id,
                PlayerActionType::None,
                PlayerActionLayer::Base,
                PlayerActionResult::Cancel,
            );
            _ = player_clear_action_state::reduce(
                ctx,
                player_entity_id,
                PlayerActionType::None,
                PlayerActionLayer::UpperBody,
                PlayerActionResult::Cancel,
            );

            ctx.db.signed_in_player_state().entity_id().delete(&player_entity_id);

            let player_dimension = ctx.db.mobile_entity_state().entity_id().find(player_entity_id).unwrap().dimension;
            PlayerHousingState::update_is_empty_flag(ctx, player_dimension);

            log::info!("Signout complete for {:?}", identity.to_hex());
        } else {
            log::info!("(sign_out) No player for identity for {:?}", identity.to_hex());
        }
    } else {
        log::info!("(sign_out) No user for identity for {:?}", identity.to_hex());
    }
}

fn cancel_trade(ctx: &ReducerContext, mut trade: TradeSessionState) {
    if trade.status != TradeSessionStatus::SessionResolved {
        trade.resolution_message = "Player disconnected".to_string();
        if let Err(err) = trade.cancel_session_and_update(ctx) {
            spacetimedb::log::error!("Failed to cancel trade: {err}");
        }
    }
}
