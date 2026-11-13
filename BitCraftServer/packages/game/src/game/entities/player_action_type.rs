use spacetimedb::ReducerContext;

use crate::{messages::components::*, player_action_desc};

impl PlayerActionType {
    pub fn get_layer(&self, ctx: &ReducerContext) -> PlayerActionLayer {
        match ctx.db.player_action_desc().action_type_id().find(*self as i32 ) {
            Some(x) => x.layer,
            None => PlayerActionLayer::Base,
        }
    }
}
