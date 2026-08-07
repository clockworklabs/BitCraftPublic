use spacetimedb::{Identity, ReducerContext};

use crate::{
    messages::{
        generic::globals,
        inter_module::{MessageContentsV5, UserUpdateRegionMsg},
    },
    unwrap_or_err,
};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    let region_id = unwrap_or_err!(ctx.db.globals().version().find(0), "Globals does not exist").region_index;
    let msg = UserUpdateRegionMsg { identity, region_id };

    send_inter_module_message(
        ctx,
        MessageContentsV5::UserUpdateRegionRequest(msg),
        super::InterModuleDestination::Global,
    );

    Ok(())
}
