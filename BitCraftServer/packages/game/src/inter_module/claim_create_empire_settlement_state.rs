use spacetimedb::ReducerContext;

use crate::{
    game::coordinates::OffsetCoordinatesSmall,
    messages::inter_module::{ClaimCreateEmpireSettlementMsg, MessageContentsV5},
};

use super::send_inter_module_message;

pub fn send_message(ctx: &ReducerContext, claim_entity_id: u64, building_entity_id: u64, location: OffsetCoordinatesSmall) {
    let msg = ClaimCreateEmpireSettlementMsg {
        claim_entity_id,
        building_entity_id,
        location,
    };

    send_inter_module_message(
        ctx,
        MessageContentsV5::ClaimCreateEmpireSettlementState(msg),
        super::InterModuleDestination::Global,
    );
}
