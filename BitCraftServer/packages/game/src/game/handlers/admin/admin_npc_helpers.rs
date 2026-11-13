use spacetimedb::{log, ReducerContext, Table};

use crate::{
    building_state, dimension_description_state,
    game::{autogen::_delete_entity::delete_entity, entities::location::LocationState, handlers::authentication::has_role},
    interior_instance_desc, interior_spawn_desc, location_state,
    messages::{authentication::Role, components::NpcState, static_data::NpcType},
    npc_state, trade_order_state,
};

#[spacetimedb::reducer]
pub fn delete_all_npcs(ctx: &ReducerContext) {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return;
    }

    for npc in ctx.db.npc_state().iter() {
        for trade_order in ctx.db.trade_order_state().shop_entity_id().filter(npc.building_entity_id) {
            delete_entity(ctx, trade_order.entity_id);
        }
        delete_entity(ctx, npc.entity_id);
    }
}

#[spacetimedb::reducer]
pub fn respawn_interior_npcs(ctx: &ReducerContext) {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return;
    }

    for dimension_description in ctx.db.dimension_description_state().iter() {
        if let Some(interior) = ctx
            .db
            .interior_instance_desc()
            .id()
            .find(dimension_description.interior_instance_id)
        {
            for spawn in ctx.db.interior_spawn_desc().interior_instance_id().filter(interior.id) {
                let traveler_type = spawn.traveler_type;
                if traveler_type != NpcType::None {
                    let dimension = dimension_description.dimension_id;
                    for loc in LocationState::select_all_in_interior_dimension_iter(ctx, dimension) {
                        if let Some(building) = ctx.db.building_state().entity_id().find(loc.entity_id) {
                            if building.building_description_id == spawn.traveler_ruin_entity_id
                                && ctx.db.npc_state().building_entity_id().filter(building.entity_id).next().is_none() {
                                    let offset_coord = ctx
                                        .db
                                        .location_state()
                                        .entity_id()
                                        .find(building.entity_id)
                                        .unwrap()
                                        .offset_coordinates();
                                    NpcState::spawn(
                                        ctx,
                                        traveler_type,
                                        building.direction_index,
                                        building.entity_id,
                                        offset_coord,
                                        false,
                                    );
                                    log::info!("Respawned {:?} in dimension {} interior", traveler_type, dimension);
                                }
                        }
                    }
                }
            }
        }
    }
}
