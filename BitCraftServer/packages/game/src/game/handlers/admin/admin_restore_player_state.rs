use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::game::game_state;
use crate::{
    deployable_state_v2,
    game::{
        handlers::{authentication::has_role, player::sign_out::sign_out_internal},
        reducer_helpers::{
            deployable_helpers::store_deployable, player_action_helpers::post_reducer_update_cargo,
            restore_player_helpers::auto_unstuck_player_and_deployable, timer_helpers::now_plus_millis,
        },
    },
    inventory_state,
    messages::{authentication::Role, game_util::ItemType},
    mobile_entity_state, player_lowercase_username_state, user_state, InventoryState, PlayerActionLayer, PlayerActionState,
    PlayerActionType, PlayerState,
};
use crate::messages::events::{player_signed_out_event, PlayerSignedOutEvent, PlayerSignedOutReason};

#[spacetimedb::table(accessor = admin_restore_player_state_timer, scheduled(admin_restore_player_state_scheduled, at = scheduled_at))]
pub struct AdminRestorePlayerStateTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    username: String,
    restore_position: bool,
    force_signout: bool,
    restore_all_deployables_positions: bool,
    store_deployables: bool,
    clear_cargo: bool,
    clear_items: bool,
    clear_toolbelt: bool,
}

#[spacetimedb::reducer]
pub fn admin_restore_player_state_scheduled(ctx: &ReducerContext, timer: AdminRestorePlayerStateTimer) -> Result<(), String> {
    admin_restore_player_state(
        ctx,
        timer.username,
        timer.restore_position,
        timer.force_signout,
        timer.restore_all_deployables_positions,
        timer.store_deployables,
        timer.clear_cargo,
        timer.clear_items,
        timer.clear_toolbelt,
    )
}

#[spacetimedb::reducer]
#[shared_table_reducer]
#[expect(clippy::too_many_arguments, reason = "reducer arguments are part of the client API")]
pub fn admin_restore_player_state(
    ctx: &ReducerContext,
    username: String,
    restore_position: bool,
    force_signout: bool,
    restore_all_deployables_positions: bool,
    store_deployables: bool,
    clear_cargo: bool,
    clear_items: bool,
    clear_toolbelt: bool,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        log::error!("Unauthorized.");
        return Ok(());
    }

    let entity_id = match ctx
        .db
        .player_lowercase_username_state()
        .username_lowercase()
        .find(username.to_lowercase().to_string())
    {
        Some(player) => player.entity_id,
        None => return Err(format!("Could not find player with username {}", username)),
    };

    if force_signout {
        //Sign player out and reschedule this reducer after a short delay
        let identity = ctx.db.user_state().entity_id().find(entity_id).unwrap().identity;
        sign_out_internal(ctx, identity, true);
        ctx.db.player_signed_out_event().insert(PlayerSignedOutEvent {
            identity,
            reason: PlayerSignedOutReason::AdminAction,
        });
        ctx.db
            .admin_restore_player_state_timer()
            .try_insert(AdminRestorePlayerStateTimer {
                scheduled_id: 0,
                scheduled_at: now_plus_millis(500, ctx.timestamp),
                username,
                restore_position,
                force_signout: false,
                restore_all_deployables_positions,
                store_deployables,
                clear_cargo,
                clear_items,
                clear_toolbelt,
            })
            .ok()
            .unwrap();
        return Ok(());
    }

    if restore_position || restore_all_deployables_positions {
        auto_unstuck_player_and_deployable(ctx, &entity_id);
    }

    if store_deployables || restore_all_deployables_positions {
        let player_mobile_entity_state = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap();

        for deployable_state in ctx.db.deployable_state_v2().owner_id().filter(entity_id) {
            let mut deployable_mobile_entity_state = ctx.db.mobile_entity_state().entity_id().find(deployable_state.entity_id).unwrap();
            deployable_mobile_entity_state.set_location(player_mobile_entity_state.offset_coordinates_float());
            deployable_mobile_entity_state.set_destination(player_mobile_entity_state.offset_destination_float());
            ctx.db.mobile_entity_state().entity_id().update(deployable_mobile_entity_state);
        }

        if store_deployables {
            for deployable_state in ctx.db.deployable_state_v2().owner_id().filter(entity_id) {
                if ctx.db.mobile_entity_state().entity_id().find(deployable_state.entity_id).is_some() {
                    let _ = store_deployable(ctx, entity_id, deployable_state.entity_id, false);
                }
            }
        }
    }

    if clear_items || clear_cargo {
        let mut inventory = InventoryState::get_player_inventory(ctx, entity_id).unwrap();
        for pocket in &mut inventory.pockets {
            if let Some(contents) = pocket.contents {
                if (contents.item_type == ItemType::Cargo && clear_cargo) || (contents.item_type == ItemType::Item && clear_items) {
                    pocket.contents = None;
                    pocket.locked = false;
                }
            }
        }
        ctx.db.inventory_state().entity_id().update(inventory);
    }

    if clear_toolbelt {
        let mut toolbelt_inventory = InventoryState::get_player_toolbelt(ctx, entity_id).unwrap();
        for pocket in &mut toolbelt_inventory.pockets {
            pocket.contents = None;
            pocket.locked = false;
        }
        ctx.db.inventory_state().entity_id().update(toolbelt_inventory);
    }

    //Clear PlayerActionState
    PlayerActionState::success(
        ctx,
        entity_id,
        PlayerActionType::None,
        PlayerActionLayer::Base,
        0,
        None,
        None,
        game_state::unix_ms(ctx.timestamp),
    );
    PlayerActionState::success(
        ctx,
        entity_id,
        PlayerActionType::None,
        PlayerActionLayer::UpperBody,
        0,
        None,
        None,
        game_state::unix_ms(ctx.timestamp),
    );

    //Update buffs
    post_reducer_update_cargo(ctx, entity_id);
    PlayerState::collect_stats(ctx, entity_id);

    Ok(())
}
