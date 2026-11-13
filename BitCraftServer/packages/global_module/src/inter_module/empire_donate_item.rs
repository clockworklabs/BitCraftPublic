use spacetimedb::ReducerContext;

use crate::{
    messages::{
        components::player_username_state,
        empire_schema::EmpireNotificationState,
        empire_shared::*,
        inter_module::*,
        static_data::{item_desc, EmpireNotificationType},
    },
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireDonateItemMsg) -> Result<(), String> {
    let player_empire_data = unwrap_or_err!(
        ctx.db.empire_player_data_state().entity_id().find(request.player_entity_id),
        "Player must be part of an empire to donate"
    );

    let mut empire = unwrap_or_err!(
        ctx.db.empire_state().entity_id().find(player_empire_data.empire_entity_id),
        "This empire does not exist"
    );

    let mut player_data = unwrap_or_err!(
        ctx.db.empire_player_data_state().entity_id().find(request.player_entity_id),
        "You are not part of an empire"
    );

    let donator_name = ctx
        .db
        .player_username_state()
        .entity_id()
        .find(request.player_entity_id)
        .unwrap()
        .username;
    if let Some(ref on_behalf) = request.on_behalf_username {
        let on_behalf_state = unwrap_or_err!(ctx.db.player_username_state().username().find(on_behalf), "Player does not exist");
        player_data = unwrap_or_err!(
            ctx.db.empire_player_data_state().entity_id().find(on_behalf_state.entity_id),
            "That player is not part of an empire"
        );
        if player_data.empire_entity_id != empire.entity_id {
            return Err("That player is not part of your empire".into());
        }
    }

    player_data.donated_empire_currency += request.count;

    // Citizens-to-Noble auto-upgrade
    if player_data.donated_shards + player_data.donated_empire_currency >= empire.nobility_threshold as u32 && player_data.noble.is_none() {
        player_data.noble = Some(ctx.timestamp);
        if player_data.rank == 9 {
            player_data.rank = 8;
        }
    }
    EmpirePlayerDataState::update_shared(ctx, player_data, crate::inter_module::InterModuleDestination::AllOtherRegions);

    if request.is_cargo {
        // for now no cargo can be donated
        return Err("This cargo can't be donated".into());
    } else {
        let item_desc = unwrap_or_err!(ctx.db.item_desc().id().find(request.item_id), "Unknown donated item");
        match item_desc.tag.as_str() {
            "Empire Currency" => {
                if u32::MAX - empire.empire_currency_treasury < request.count {
                    return Err("Currency Overflow".into());
                }
                empire.empire_currency_treasury += request.count;
            }
            _ => return Err("This item can't be donated".into()),
        }
    }

    if let Some(on_behalf_name) = request.on_behalf_username {
        // Donation On Behalf Notification (14)
        EmpireNotificationState::new(
            ctx,
            EmpireNotificationType::DonationByProxy,
            empire.entity_id,
            vec![donator_name, format!("{}", request.count), on_behalf_name],
        );
    } else {
        // Donation Notification (13)
        EmpireNotificationState::new(
            ctx,
            EmpireNotificationType::Donation,
            empire.entity_id,
            vec![donator_name, format!("{}", request.count)],
        );
    }

    EmpireState::update_shared(ctx, empire, super::InterModuleDestination::AllOtherRegions);
    EmpireSettlementState::update_donations_from_player(ctx, request.player_entity_id, false)?;

    Ok(())
}
