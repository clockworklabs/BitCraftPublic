use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    dimension_description_state,
    game::{
        coordinates::{HexDirection, OffsetCoordinatesSmall, SmallHexTile},
        reducer_helpers::interior_helpers::spawn_interior_resource,
    },
    interior_instance_desc, interior_shape_desc, interior_spawn_desc, location_state,
    messages::{authentication::Role, components::ResourceState, static_data::InteriorSpawnType},
    resource_clump_desc, resource_state, unwrap_or_err,
};

use crate::game::handlers::authentication::has_role;

/// Force-restores one resource InteriorSpawnDesc in every live matching interior.
/// Existing resources at the desc's anchor are removed first, then the normal
/// interior spawn path recreates the full resource clump and collapse trigger.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_respawn_interior_spawn_desc(ctx: &ReducerContext, interior_spawn_desc_id: i32, commit: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let spawn = unwrap_or_err!(
        ctx.db.interior_spawn_desc().id().find(&interior_spawn_desc_id),
        "Invalid InteriorSpawnDesc"
    );
    if spawn.spawn_type != InteriorSpawnType::Resource {
        return Err("This recovery reducer currently supports only resource InteriorSpawnDesc records".into());
    }
    let instance = unwrap_or_err!(
        ctx.db.interior_instance_desc().id().find(&spawn.interior_instance_id),
        "Invalid interior instance"
    );
    let shape = unwrap_or_err!(
        ctx.db.interior_shape_desc().id().find(&instance.interior_shape_id),
        "Invalid interior shape"
    );
    let resource_clump = unwrap_or_err!(
        ctx.db.resource_clump_desc().id().find(&spawn.resource_clump_id),
        "Invalid resource clump"
    );
    let dimensions: Vec<_> = ctx
        .db
        .dimension_description_state()
        .iter()
        .filter(|dimension| dimension.interior_instance_id == spawn.interior_instance_id)
        .collect();

    log::info!(
        "Found {} live interiors for InteriorSpawnDesc {}",
        dimensions.len(),
        interior_spawn_desc_id
    );
    if !commit {
        return Ok(());
    }

    for dimension in dimensions {
        let location = OffsetCoordinatesSmall {
            x: spawn.spawn_x - shape.min_x,
            z: spawn.spawn_z - shape.min_z,
            dimension: dimension.dimension_id,
        };
        let anchor = SmallHexTile::from(location);
        let resource_centers: Vec<SmallHexTile> = resource_clump
            .x
            .iter()
            .zip(&resource_clump.z)
            .map(|(x, z)| {
                SmallHexTile {
                    x: anchor.x + x,
                    z: anchor.z + z,
                    dimension: anchor.dimension,
                }
                .rotate_around(&anchor, (HexDirection::from(spawn.direction) as i32) / 2)
            })
            .collect();
        let existing_resource_ids: Vec<u64> = ctx
            .db
            .location_state()
            .iter()
            .filter(|location| resource_centers.contains(&location.coordinates()))
            .filter_map(|location| {
                ctx.db
                    .resource_state()
                    .entity_id()
                    .find(&location.entity_id)
                    .map(|resource| resource.entity_id)
            })
            .collect();
        for entity_id in existing_resource_ids {
            ResourceState::despawn(
                ctx,
                entity_id,
                ctx.db.resource_state().entity_id().find(&entity_id).unwrap().resource_id,
            );
        }
        spawn_interior_resource(ctx, &spawn, location, dimension.dimension_network_entity_id);
    }

    log::info!(
        "Restored InteriorSpawnDesc {} in all live matching interiors",
        interior_spawn_desc_id
    );
    Ok(())
}
