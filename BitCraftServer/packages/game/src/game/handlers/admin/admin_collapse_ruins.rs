use spacetimedb::{ReducerContext, Table};

use crate::{
    building_state, dimension_network_state,
    game::{handlers::authentication::has_role, reducer_helpers::interior_helpers::interior_trigger_collapse},
    interior_network_desc,
    messages::{authentication::Role, game_util::DimensionType},
    unwrap_or_err, DimensionNetworkState,
};

#[spacetimedb::reducer]
pub fn admin_collapse_ruins(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    for dnd in ctx.db.dimension_network_state().iter() {
        if let Err(s) = trigger_interior_collapse(ctx, &dnd) {
            spacetimedb::log::error!("Error triggering collapse of interior {}: {}", dnd.building_id, s);
        }
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_collapse_ruin(ctx: &ReducerContext, ruin_building_entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let dnd = unwrap_or_err!(
        ctx.db.dimension_network_state().building_id().find(ruin_building_entity_id),
        "Building doesn't have an interior"
    );

    trigger_interior_collapse(ctx, &dnd)
}

fn trigger_interior_collapse(ctx: &ReducerContext, dimension_network: &DimensionNetworkState) -> Result<(), String> {
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(dimension_network.building_id),
        "Building doesn't exist"
    );
    let ind = unwrap_or_err!(
        ctx.db.interior_network_desc().building_id().find(building.building_description_id),
        "Building doesn't have an interior"
    );
    if ind.dimension_type != DimensionType::AncientRuin && ind.dimension_type != DimensionType::Dungeon {
        return Ok(());
    }

    interior_trigger_collapse(ctx, dimension_network.entity_id)
}
