use spacetimedb::{Identity, ReducerContext};

use crate::{
    game::handlers::authentication::has_role,
    inter_module::send_inter_module_message,
    messages::{
        authentication::Role,
        components::{player_lowercase_username_state, user_state},
        global::user_region_state,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
fn admin_skip_queue_identity(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err("Unauthorized".into());
    }

    return reduce(ctx, identity);
}

#[spacetimedb::reducer]
fn admin_skip_queue_entity(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err("Unauthorized".into());
    }

    let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(entity_id), "User not found");
    return reduce(ctx, user.identity);
}

#[spacetimedb::reducer]
fn admin_skip_queue_name(ctx: &ReducerContext, name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err("Unauthorized".into());
    }

    let name_state = unwrap_or_err!(
        ctx.db
            .player_lowercase_username_state()
            .username_lowercase()
            .find(name.to_lowercase().to_string()),
        "User not found"
    );
    let user = unwrap_or_err!(ctx.db.user_state().entity_id().find(name_state.entity_id), "User not found");
    return reduce(ctx, user.identity);
}

fn reduce(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    let user_region = unwrap_or_err!(ctx.db.user_region_state().identity().find(identity), "User not found");
    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV5::PlayerSkipQueue(crate::messages::inter_module::PlayerSkipQueueMsg {
            player_identity: identity,
        }),
        crate::inter_module::InterModuleDestination::Region(user_region.region_id),
    );

    Ok(())
}
