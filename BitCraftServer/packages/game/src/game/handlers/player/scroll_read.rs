use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{discovery::Discovery, entities::building_state::InventoryState, game_state},
    knowledge_scroll_desc,
    messages::{action_request::PlayerScrollReadRequest, components::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn scroll_read(ctx: &ReducerContext, request: PlayerScrollReadRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    let pocket_index = request.pocket_index as usize;
    let contents = unwrap_or_err!(inventory.get_pocket_contents(pocket_index), "Invalid pocket");
    let item_id = contents.item_id;
    if item_id <= 0 {
        return Err("Nothing to read".into());
    }

    let scroll = unwrap_or_err!(ctx.db.knowledge_scroll_desc().item_id().find(item_id), "Not a scroll");

    inventory.remove_quantity_at(pocket_index, 1);
    ctx.db.inventory_state().entity_id().update(inventory);

    //Check if player already knows this knowledge
    if Discovery::has_player_acquired_lore(ctx, actor_id, scroll.item_id) {
        return Err("You already know this lore.".into());
    }

    let mut discovery = Discovery::new(actor_id);

    discovery.acquire_lore(ctx, scroll.item_id);

    if scroll.secondary_knowledge_id != 0 {
        discovery.acquire_secondary(ctx, scroll.secondary_knowledge_id);
    }

    discovery.commit(ctx);

    Ok(())
}
