use spacetimedb::ReducerContext;

use crate::messages::{
    components::{player_lowercase_username_state, player_username_state},
    events::PlayerSetNameOutcomeEvent,
    inter_module::OnPlayerNameSetMsg,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: OnPlayerNameSetMsg) -> Result<(), String> {
    let entity_id = request.player_entity_id;
    let username = request.name;

    let mut player_state = ctx.db.player_lowercase_username_state().entity_id().find(entity_id).unwrap();
    player_state.username_lowercase = username.to_lowercase();
    ctx.db.player_lowercase_username_state().entity_id().update(player_state);

    let mut player_state = ctx.db.player_username_state().entity_id().find(entity_id).unwrap();
    player_state.username = username.clone();
    ctx.db.player_username_state().entity_id().update(player_state);

    PlayerSetNameOutcomeEvent::new_event(ctx, entity_id);

    Ok(())
}
