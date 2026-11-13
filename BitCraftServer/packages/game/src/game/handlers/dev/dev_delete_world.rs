use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{autogen::_delete_entity::clear_entity, handlers::authentication::has_role, world_gen::resources_log::resources_log},
    herd_state, location_cache,
    messages::{
        authentication::Role,
        generic::{config, globals},
    },
    terrain_chunk_state, user_state,
};

#[spacetimedb::reducer]
pub fn dev_delete_world(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    spacetimedb::log::info!("dev_delete_world triggered by {}", ctx.sender());

    //Delete users
    let mut users = Vec::with_capacity(100);
    for user in ctx.db.user_state().iter() {
        users.push(user.entity_id);
    }
    for user in users {
        ctx.db.user_state().entity_id().delete(user);
    }

    //Delete chunks
    let mut chunks = Vec::with_capacity(100);
    chunks.extend(ctx.db.terrain_chunk_state().iter().map(|c| c.chunk_index));
    for chunk in chunks {
        ctx.db.terrain_chunk_state().chunk_index().delete(chunk);
    }

    //Clear caches
    let mut herds = Vec::with_capacity(100);
    for herd in ctx.db.herd_state().iter() {
        herds.push(herd.entity_id);
    }
    for herd in herds {
        ctx.db.herd_state().entity_id().delete(herd);
    }
    ctx.db.location_cache().version().delete(0);
    ctx.db.resources_log().version().delete(0);

    //Delete entities
    let mut globals = ctx.db.globals().version().find(0).unwrap();
    for i in 0..=globals.entity_pk_counter {
        clear_entity(ctx, i);
    }

    globals.entity_pk_counter = 0;
    globals.dimension_counter = 0;
    ctx.db.globals().version().update(globals);

    let mut config = ctx.db.config().version().find(0).unwrap();
    config.agents_enabled = false;
    ctx.db.config().version().update(config);

    spacetimedb::log::info!("World deleted");

    Ok(())
}
