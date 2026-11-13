use spacetimedb::ReducerContext;

use crate::{
    game::{handlers::buildings::project_site_place::refund_recipe_input, reducer_helpers::building_helpers::delete_building},
    messages::{components::{NotificationSeverity, PlayerNotificationEvent}, inter_module::EmpireCreateBuildingMsg},
};

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: EmpireCreateBuildingMsg, error: Option<String>) {
    if let Some(error) = error {
        //Destroy building and refund construction mats if call fails
        delete_building(ctx, request.player_entity_id, request.building_entity_id, None, false, false);
        refund_recipe_input(ctx, request.player_entity_id, request.construction_recipe_id);
        PlayerNotificationEvent::new_event(ctx, request.player_entity_id, error, NotificationSeverity::ReducerError);
    }
}
