use spacetimedb::ReducerContext;

use crate::{
    game::{game_state, handlers::authentication::has_role},
    inter_module::{send_inter_module_message, InterModuleDestination},
    messages::{
        authentication::Role,
        components::{
            player_lowercase_username_state, player_username_state, previous_player_username_state, user_state,
            PlayerLowercaseUsernameState, PlayerUsernameState,
        },
        inter_module::{MessageContentsV5, OnPlayerNameSetMsg},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
pub fn admin_rename_player(ctx: &ReducerContext, current_name: String, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err("Unauthorized".into());
    }

    let entity_id = unwrap_or_err!(
        ctx.db
            .player_lowercase_username_state()
            .username_lowercase()
            .find(current_name.to_lowercase()),
        "Player not found"
    )
    .entity_id;

    admin_rename_player_entity(ctx, entity_id, new_name)
}

#[spacetimedb::reducer]
pub fn admin_rename_player_entity(ctx: &ReducerContext, entity_id: u64, new_name: String) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Gm) {
        return Err("Unauthorized".into());
    }

    let user = ctx.db.user_state().entity_id().find(entity_id).unwrap();

    // Prevent the player from using a name assigned to a different player
    if let Some(previous_entry) = ctx
        .db
        .previous_player_username_state()
        .lower_case_name()
        .find(new_name.to_lowercase())
    {
        if previous_entry.identity != user.identity {
            return Err("This name is unavailable".into());
        }
    }

    // Clear player's reserved name, if any, whether it used the previous name or not
    ctx.db.previous_player_username_state().identity().delete(user.identity);

    ctx.db
        .player_lowercase_username_state()
        .entity_id()
        .update(PlayerLowercaseUsernameState {
            entity_id,
            username_lowercase: new_name.to_lowercase(),
        });
    ctx.db.player_username_state().entity_id().update(PlayerUsernameState {
        entity_id,
        username: new_name.clone(),
    });

    let msg = OnPlayerNameSetMsg {
        player_entity_id: entity_id,
        name: new_name,
    };
    let player_region = game_state::player_region(ctx, entity_id)?;
    send_inter_module_message(
        ctx,
        MessageContentsV5::OnPlayerNameSetRequest(msg),
        InterModuleDestination::Region(player_region),
    );

    Ok(())
}
