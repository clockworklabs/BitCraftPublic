use crate::{
    game::handlers::authentication::has_role,
    messages::{
        authentication::Role,
        components::player_lowercase_username_state,
        global::{player_developer_notification_state, PlayerDeveloperNotificationState},
    },
    unwrap_or_err, user_state,
};
use spacetimedb::{Identity, ReducerContext, Table};
use std::str::FromStr;

#[spacetimedb::reducer]
pub fn admin_notify_player_by_identity(ctx: &ReducerContext, identity: String, title: String, message: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let identity = Identity::from_str(identity.as_str());
    if identity.is_err() {
        return Err("Identity couldn't be parsed".into());
    }
    let identity = identity.unwrap();

    // If player is signed-in, directly update their vault
    let user = unwrap_or_err!(ctx.db.user_state().identity().find(identity), "Player does not exist");

    reduce(ctx, user.entity_id, title, message)
}

#[spacetimedb::reducer]
pub fn admin_notify_player(ctx: &ReducerContext, username: String, title: String, message: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let lowercase_username = username.to_lowercase();

    let player = unwrap_or_err!(
        ctx.db
            .player_lowercase_username_state()
            .username_lowercase()
            .find(lowercase_username),
        "Player does not exist"
    );

    reduce(ctx, player.entity_id, title, message)
}

fn reduce(ctx: &ReducerContext, entity_id: u64, title: String, message: String) -> Result<(), String> {
    if let Some(mut player_notification) = ctx.db.player_developer_notification_state().entity_id().find(entity_id) {
        player_notification.message = message;
        player_notification.title = title;
        ctx.db.player_developer_notification_state().entity_id().update(player_notification);
    } else {
        let player_notification = PlayerDeveloperNotificationState { entity_id, title, message };
        ctx.db.player_developer_notification_state().insert(player_notification);
    }
    Ok(())
}
