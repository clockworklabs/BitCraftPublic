use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{game_state, reducer_helpers::player_action_helpers},
    messages::{
        action_request::PlayerOrderCollectRequest,
        components::{closed_listing_state, AuctionListingState, HealthState, InventoryState},
    },
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("trade")]
pub fn order_collect(ctx: &ReducerContext, request: PlayerOrderCollectRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let building = AuctionListingState::marketplace_for_action(ctx, actor_id, request.building_entity_id, 2)?;

    let claim_entity_id = building.claim_entity_id;

    let mut order = unwrap_or_err!(
        ctx.db.closed_listing_state().entity_id().find(request.closed_listing_entity_id),
        "This listing does not exist"
    );

    if order.claim_entity_id != claim_entity_id {
        return Err("You cannot collect this listing from there".into());
    }

    if order.owner_entity_id != actor_id {
        return Err("You are not the owner of this listing".into());
    }

    let item_stacks = vec![order.item_stack];

    let remaining_items = InventoryState::deposit_to_player_inventory_and_nearby_deployables_and_get_overflow_stack(
        ctx,
        actor_id,
        &item_stacks,
        |x| building.distance_to(ctx, &x),
        true,
    )?;

    if remaining_items.is_empty() {
        ctx.db.closed_listing_state().entity_id().delete(order.entity_id);
    } else {
        order.item_stack = *remaining_items.first().unwrap();
        ctx.db.closed_listing_state().entity_id().update(order);
    }

    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
