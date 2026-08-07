use spacetimedb::ReducerContext;

use crate::messages::{
    global::user_region_state,
    inter_module::{MessageContentsV5, PlayerCreateMsg},
};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, region_id: u8) -> Result<(), String> {
    let msg = PlayerCreateMsg { identity: ctx.sender };

    send_inter_module_message(
        ctx,
        MessageContentsV5::PlayerCreateRequest(msg),
        super::InterModuleDestination::Region(region_id),
    );

    return Ok(());
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: PlayerCreateMsg, error: Option<String>) {
    if error.is_some() {
        ctx.db.user_region_state().identity().delete(request.identity);
    }
}
