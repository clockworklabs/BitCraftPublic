use spacetimedb::ReducerContext;

use crate::{
    game::coordinates::SmallHexTile,
    messages::{
        components::TerrainChunkState,
        empire_shared::{empire_settlement_state, EmpireSettlementState},
        inter_module::ClaimCreateEmpireSettlementMsg,
    },
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: ClaimCreateEmpireSettlementMsg) -> Result<(), String> {
    if ctx
        .db
        .empire_settlement_state()
        .claim_entity_id()
        .find(request.claim_entity_id)
        .is_none()
    {
        let chunk_index = TerrainChunkState::chunk_index_from_coords(&SmallHexTile::from(request.location).chunk_coordinates());
        let settlement = EmpireSettlementState {
            building_entity_id: request.building_entity_id,
            claim_entity_id: request.claim_entity_id,
            empire_entity_id: 0,
            chunk_index,
            can_house_empire_storehouse: false,
            members_donations: 0,
            location: request.location,
        };
        EmpireSettlementState::insert_shared(ctx, settlement, super::InterModuleDestination::AllOtherRegions);
    }

    Ok(())
}
