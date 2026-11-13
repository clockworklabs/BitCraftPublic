use bitcraft_macro::feature_gate;
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

use crate::game::game_state;
use crate::messages::action_request::ReportEntityMessage;
use crate::messages::components::*;
use crate::messages::empire_shared::{empire_rank_state, empire_state};
use crate::messages::static_data::building_desc;
use crate::unwrap_or_err;

#[spacetimedb::reducer]
#[shared_table_reducer]
#[feature_gate]
pub fn report_entity(ctx: &ReducerContext, request: ReportEntityMessage) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    //find entity name
    let entity_name = match request.report_type.as_str() {
        "Building Name" => {
            let nickname = ctx
                .db
                .building_nickname_state()
                .entity_id()
                .find(request.entity_id)
                .map(|b| b.nickname.clone())
                .unwrap_or_default();

            if nickname.trim().is_empty() {
                let building_desc_id = unwrap_or_err!(ctx.db.building_state().entity_id().find(request.entity_id), "building not found")
                    .building_description_id;

                unwrap_or_err!(ctx.db.building_desc().id().find(building_desc_id), "building not found")
                    .name
                    .clone()
            } else {
                nickname
            }
        }
        "Sign Text" => {
            unwrap_or_err!(ctx.db.player_note_state().entity_id().find(request.entity_id), "Sign not found").text
        }
        "Empire Name" => {
            unwrap_or_err!(ctx.db.empire_state().entity_id().find(request.entity_id), "Empire not found").name
        }
        "Empire Rank Name" => {
            unwrap_or_err!(
                ctx.db.empire_rank_state().entity_id().find(request.entity_id),
                "Empire rank not found"
            )
            .title
        }
        "Claim Name" => {
            unwrap_or_err!(ctx.db.claim_state().entity_id().find(request.entity_id), "Claim not found").name
        }
        _ => {
            "Unknown".to_string()
        }
    };

    let entity_id = game_state::create_entity(ctx);
    PlayerReportState::insert_shared(
        ctx,
        PlayerReportState {
            entity_id,
            reporter_entity_id: actor_id,
            reported_player_entity_id: request.entity_id,
            reported_player_username: entity_name,
            report_type: request.report_type,
            report_message: request.message,
            reported_chat_message: None,
            chat_channel_context: None,
            chat_user_context: None,
            actioned: false,
        },
        crate::inter_module::InterModuleDestination::Global,
    );
    ctx.db.player_report_state().entity_id().delete(entity_id); //We don't actually need this report locally

    Ok(())
}
