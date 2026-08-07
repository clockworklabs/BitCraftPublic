use spacetimedb::{Identity, ReducerContext};

use crate::{
    messages::{global::user_region_state, inter_module::*},
    unwrap_or_err,
};

use super::{send_inter_module_message, InterModuleDestination};

pub fn send_message(ctx: &ReducerContext, player_identity: Identity) -> Result<(), String> {
    let region = unwrap_or_err!(ctx.db.user_region_state().identity().find(player_identity), "User region not found").region_id;
    send_inter_module_message(
        ctx,
        MessageContentsV5::SignPlayerOut(SignPlayerOutMsg {
            player_identity: player_identity,
        }),
        InterModuleDestination::Region(region),
    );
    Ok(())
}
