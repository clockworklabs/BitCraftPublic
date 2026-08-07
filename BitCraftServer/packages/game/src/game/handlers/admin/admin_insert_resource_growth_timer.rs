use crate::{
    game::{entities::growth_timer::insert_resource_growth_timer, handlers::authentication::has_role},
    messages::{
        authentication::Role,
        components::resource_state,
        static_data::resource_growth_recipe_desc,
    },
    unwrap_or_err,
};
use spacetimedb::{log, ReducerContext};

#[spacetimedb::reducer]
pub fn admin_insert_resource_growth_timer(ctx: &ReducerContext, resource_id: i32) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let growth = unwrap_or_err!(
        ctx.db.resource_growth_recipe_desc().resource_id().filter(resource_id).last(),
        "Unknown ResourceGrowthRecipeDesc "
    );

    let mut count = 0;
    for resource in ctx.db.resource_state().resource_id().filter(resource_id) {
        if insert_resource_growth_timer(ctx, resource.entity_id, &growth).is_ok() {
            count += 1;
        }
    }

    log::info!("Inserted {} ResourceGrowthTimers for resource with id {}", count, resource_id);
    Ok(())
}
