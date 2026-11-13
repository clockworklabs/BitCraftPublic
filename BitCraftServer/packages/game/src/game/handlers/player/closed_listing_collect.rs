use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{discovery::Discovery, game_state},
    messages::{
        action_request::PlayerClosedListingCollectRequest,
        components::{building_state, closed_listing_state, mobile_entity_state, HealthState, InventoryState},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn closed_listing_collect(ctx: &ReducerContext, request: PlayerClosedListingCollectRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(request.building_entity_id),
        "Building does not exist"
    );

    let coordinates = ctx.db.mobile_entity_state().entity_id().find(actor_id).unwrap().coordinates();
    if building.distance_to(ctx, &coordinates) > 5 {
        return Err("Too far".into());
    }

    let claim_entity_id = building.claim_entity_id;

    // Find all sell_orders matching the price and item id, and collect items from those, sorted by increasing price (with timestamp for tie-breaking)
    let listing = unwrap_or_err!(
        ctx.db.closed_listing_state().entity_id().find(request.auction_listing_entity_id),
        "This listing no longer exists"
    );

    if listing.owner_entity_id != actor_id {
        return Err("You are not the owner of this listing".into());
    }

    if listing.claim_entity_id != claim_entity_id {
        return Err("You cannot collect this listing from there".into());
    }

    let mut discovery = Discovery::new(actor_id);
    if !InventoryState::add_and_discover(ctx, actor_id, &mut discovery, listing.item_stack, false) {
        return Err("Unable to collect at this time".into());
    }

    discovery.commit(ctx);

    ctx.db.closed_listing_state().entity_id().delete(request.auction_listing_entity_id);

    Ok(())
}
