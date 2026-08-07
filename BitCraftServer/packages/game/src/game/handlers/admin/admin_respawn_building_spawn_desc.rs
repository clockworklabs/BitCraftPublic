use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    building_spawn_desc, building_state,
    game::{coordinates::HexDirection, game_state::game_state_filters},
    location_state,
    messages::{authentication::Role, components::ResourceState, static_data::BuildingSpawnType},
    resource_desc, resource_state, unwrap_or_err,
};

use crate::game::handlers::authentication::has_role;

/// Force-restores one resource BuildingSpawnDesc for every matching parent building.
///
/// Unlike `admin_create_building_spawns`, this reducer does not replay sibling
/// spawns, so an existing child cannot prevent the selected resource from being
/// restored. Any resources whose center is at the selected spawn point are
/// replaced with the resource specified by the desc.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_respawn_building_spawn_desc(ctx: &ReducerContext, building_spawn_desc_id: i32, commit: bool) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let spawn = unwrap_or_err!(
        ctx.db.building_spawn_desc().id().find(&building_spawn_desc_id),
        "Invalid BuildingSpawnDesc"
    );
    if spawn.spawn_type != BuildingSpawnType::Resource {
        return Err("This recovery reducer currently supports only resource BuildingSpawnDesc records".into());
    }
    let resource_id = *spawn
        .spawn_ids
        .first()
        .ok_or("BuildingSpawnDesc resource spawn has no resource id")?;
    let resource_desc = unwrap_or_err!(
        ctx.db.resource_desc().id().find(&resource_id),
        "Invalid resource id on BuildingSpawnDesc"
    );
    let building_entity_ids: Vec<u64> = ctx
        .db
        .building_state()
        .building_description_id()
        .filter(spawn.building_id)
        .map(|building| building.entity_id)
        .collect();

    log::info!(
        "Found {} parent buildings for BuildingSpawnDesc {}",
        building_entity_ids.len(),
        building_spawn_desc_id
    );

    if !commit {
        return Ok(());
    }

    for building_entity_id in building_entity_ids {
        let building = ctx.db.building_state().entity_id().find(&building_entity_id).unwrap();
        let coordinates = spawn.get_spawn_coordinates(&game_state_filters::coordinates(ctx, building_entity_id), building.direction_index);
        let existing_resource_ids: Vec<u64> = ctx
            .db
            .location_state()
            .iter()
            .filter(|location| location.coordinates() == coordinates)
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

        ResourceState::spawn(
            ctx,
            None,
            resource_id,
            coordinates,
            (HexDirection::from(spawn.direction) + HexDirection::from(building.direction_index)) as i32,
            resource_desc.max_health,
            false,
            false,
        )?;
    }

    log::info!(
        "Restored BuildingSpawnDesc {} for all matching parent buildings",
        building_spawn_desc_id
    );
    Ok(())
}
