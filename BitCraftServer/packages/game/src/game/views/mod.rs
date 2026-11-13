use spacetimedb::{view, ViewContext};

use crate::messages::components::{
    crumb_trail_exposed_state__view, player_username_state__view, prospecting_state__view, user_state__view, CrumbTrailExposedState,
    ProspectingParticipant,
};

#[view(accessor = prospecting_participants, name = "prospecting_participants", public)]
pub fn prospecting_participants(ctx: &ViewContext) -> Vec<ProspectingParticipant> {
    let mut participants = Vec::new();

    let actor_id = match ctx.db.user_state().identity().find(ctx.sender()) {
        Some(user) => user.entity_id,
        None => return participants,
    };

    if let Some(my_prospecting) = ctx.db.prospecting_state().entity_id().find(actor_id) {
        for participant in ctx
            .db
            .prospecting_state()
            .crumb_trail_entity_id()
            .filter(my_prospecting.crumb_trail_entity_id)
        {
            participants.push(ProspectingParticipant {
                player_name: ctx
                    .db
                    .player_username_state()
                    .entity_id()
                    .find(participant.entity_id)
                    .unwrap()
                    .username
                    .clone(),
                node: participant.ongoing_step,
            });
        }
    }

    participants
}

#[view(accessor = exposed_breadcrumbs, name = "exposed_breadcrumbs", public)]
pub fn exposed_breadcrumbs(ctx: &ViewContext) -> Vec<CrumbTrailExposedState> {
    let actor_id = match ctx.db.user_state().identity().find(ctx.sender()) {
        Some(user) => user.entity_id,
        None => return Vec::new(),
    };

    if let Some(my_prospecting) = ctx.db.prospecting_state().entity_id().find(actor_id) {
        let exposed_trail = ctx
            .db
            .crumb_trail_exposed_state()
            .crumb_trail_entity_id()
            .find(my_prospecting.crumb_trail_entity_id)
            .unwrap();
        return vec![exposed_trail];
    }
    Vec::new()
}
