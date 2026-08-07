use spacetimedb::ReducerContext;

use crate::messages::inter_module::{MessageContentsV5, OnClaimMembersChangedMsg};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, claim_entity_id: u64) {
    send_inter_module_message(
        ctx,
        MessageContentsV5::OnClaimMembersChanged(OnClaimMembersChangedMsg { claim_entity_id }),
        super::InterModuleDestination::Global,
    );
}
