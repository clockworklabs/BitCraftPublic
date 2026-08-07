use spacetimedb::ReducerContext;

use crate::messages::{
    components::{claim_state, BuildingNicknameState, ClaimState, NotificationSeverity, PlayerNotificationEvent},
    inter_module::{ClaimSetNameMsg, MessageContentsV5},
};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, player_entity_id: u64, claim_entity_id: u64, new_name: String) {
    send_inter_module_message(
        ctx,
        MessageContentsV5::ClaimSetName(ClaimSetNameMsg {
            player_entity_id,
            claim_entity_id,
            new_name,
        }),
        super::InterModuleDestination::Global,
    );
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: ClaimSetNameMsg, error: Option<String>) {
    if error.is_some() {
        if request.player_entity_id > 0 {
            PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error.unwrap(), NotificationSeverity::ReducerError);
        }
    } else {
        if let Some(mut claim) = ctx.db.claim_state().entity_id().find(request.claim_entity_id) {
            BuildingNicknameState::set_nickname(ctx, claim.owner_building_entity_id, request.new_name.clone());
            claim.name = request.new_name;
            ClaimState::update_shared(ctx, claim, super::InterModuleDestination::Global);
        }
    }
}
