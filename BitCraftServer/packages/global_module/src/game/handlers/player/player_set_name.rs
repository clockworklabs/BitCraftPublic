use bitcraft_macro::feature_gate;
use regex::Regex;
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{game_state, handlers::authentication::has_role},
    i18n,
    inter_module::{send_inter_module_message, InterModuleDestination},
    messages::{
        action_request::PlayerSetNameRequest,
        authentication::Role,
        components::*,
        global::user_region_state,
        inter_module::{MessageContentsV5, OnPlayerNameSetMsg},
        static_data::reserved_name_desc,
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn player_set_name(ctx: &ReducerContext, request: PlayerSetNameRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;

    if let Some(username) = ctx.db.player_username_state().entity_id().find(actor_id) {
        if username.username != format!("player{}", actor_id) {
            return Err("Player name already chosen".into());
        }
    }

    reduce(ctx, actor_id, request.username)
}

pub fn reduce(ctx: &ReducerContext, entity_id: u64, username: String) -> Result<(), String> {
    let username = username.trim().to_string();
    validate_username(ctx, entity_id, &username)?;

    let lowercase_name = username.to_lowercase();
    if !has_role(ctx, &ctx.sender, Role::Partner) {
        if lowercase_name.contains("clockwork") || lowercase_name.contains("spacetime") || lowercase_name.contains("bitcraft") {
            return Err("This name is unavailable".into());
        }
        if ctx.db.reserved_name_desc().name().find(&lowercase_name).is_some() {
            return Err("This name is unavailable".into());
        }
    }

    let identity = unwrap_or_err!(ctx.db.user_state().entity_id().find(entity_id), "Player not found").identity;

    // Prevent the player from using a name assigned to a different player
    if let Some(previous_entry) = ctx.db.previous_player_username_state().lower_case_name().find(lowercase_name) {
        if previous_entry.identity != identity {
            return Err("This name is unavailable".into());
        }
    }

    // Clear player's reserved name, if any, whether it used the previous name or not
    ctx.db.previous_player_username_state().identity().delete(identity);

    let player_region = unwrap_or_err!(ctx.db.user_region_state().identity().find(&identity), "Player region not found").region_id;

    ctx.db.player_lowercase_username_state().entity_id().delete(&entity_id);
    ctx.db.player_lowercase_username_state().insert(PlayerLowercaseUsernameState {
        entity_id,
        username_lowercase: username.to_lowercase().into(),
    });

    ctx.db.player_username_state().entity_id().delete(&entity_id);
    ctx.db.player_username_state().insert(PlayerUsernameState {
        entity_id,
        username: username.clone(),
    });

    let msg = OnPlayerNameSetMsg {
        player_entity_id: entity_id,
        name: username,
    };
    send_inter_module_message(
        ctx,
        MessageContentsV5::OnPlayerNameSetRequest(msg),
        InterModuleDestination::Region(player_region),
    );

    Ok(())
}

const MIN_LENGTH: usize = 2;
const MAX_LENGTH: usize = 16;

pub fn validate_username(ctx: &ReducerContext, entity_id: u64, username: &String) -> Result<(), String> {
    if username.len() < MIN_LENGTH {
        return Err("This username is too short.".into());
    }

    if username.len() > MAX_LENGTH {
        return Err("This username is too long.".into());
    }

    //Allows letters, accented letters and numbers
    let regex = Regex::new(r"^[\p{L}\p{N}]+$").unwrap();
    if !regex.is_match(&username) || !i18n::is_sanitized(username) {
        return Err("Invalid username.".into());
    }

    // Note A.G.: there was some code preventing only player<#> where <#> was a number different from the player_id.
    // I'm not sure behind the reasoning, considering that the user login cheat is player@<#> and not player<#>.
    // In any case, I simplified the code by preventing any name starting with "player".
    if username.starts_with("player") {
        return Err("Cannot choose that username.".into());
    }

    // TODO: Further username validation?

    if let Some(existing) = ctx
        .db
        .player_lowercase_username_state()
        .username_lowercase()
        .find(&username.to_lowercase().into())
    {
        if existing.entity_id != entity_id {
            return Err("This username is taken.".into());
        }
    }

    Ok(())
}
