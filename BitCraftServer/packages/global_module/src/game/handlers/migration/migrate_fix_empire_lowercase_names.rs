use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        empire_shared::{empire_lowercase_name_state, empire_state, EmpireLowercaseNameState},
    },
};

#[spacetimedb::reducer]
pub fn migrate_fix_empire_lowercase_names(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Gm) {
        return Err("Unauthorized".into());
    }

    let e: Vec<u64> = ctx.db.empire_lowercase_name_state().iter().map(|i| i.entity_id).collect();
    for id in e {
        ctx.db.empire_lowercase_name_state().entity_id().delete(id);
    }

    for e in ctx.db.empire_state().iter() {
        let id = e.entity_id;
        let name_lowercase = e.name.to_lowercase();
        ctx.db
            .empire_lowercase_name_state()
            .try_insert(EmpireLowercaseNameState {
                entity_id: id,
                name_lowercase,
            })
            .unwrap_or_else(|err| panic!("Empire {id} has a name that conflicts with another empire: {err:?}"));
    }

    log::info!("Empire names fixed");
    Ok(())
}
