use bitcraft_macro::feature_gate;
use crate::game::discovery::Discovery;
use crate::game::entities::building_state::InventoryState;
use crate::game::game_state;
use crate::messages::action_request::PlayerConvertDeedToCollectibleRequest;
use crate::messages::components::*;
use crate::messages::game_util::ItemType;
use crate::messages::static_data::premium_item_desc;
use crate::{collectible_desc, equipment_desc, unwrap_or_err, AchievementDesc};
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
#[feature_gate]
pub fn convert_deed_to_collectible(ctx: &ReducerContext, request: PlayerConvertDeedToCollectibleRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let mut inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");
    let mut stack = *unwrap_or_err!(inventory.get_at(request.pocket_index as usize), "Pocket cannot be empty");

    if stack.item_type != ItemType::Item {
        return Err("Deed is not an item".into());
    }

    if stack.quantity <= 0 {
        return Err("Invalid quantity".into());
    }

    let item_deed_id = stack.item_id;

    if let Some(equipment_desc) = ctx.db.equipment_desc().item_id().find(item_deed_id) {
        if !AchievementDesc::evaluate_achievements(ctx, actor_id, equipment_desc.required_achievements) {
            return Err("You don't meet the achievement requirements to convert this item into a collectible".into());
        }
    }

    let collectible = unwrap_or_err!(
        ctx.db.collectible_desc().item_deed_id().filter(item_deed_id).next(),
        "Item cannot be converted into a collectible"
    );

    for knowledge_req in &collectible.required_knowledges_to_convert {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *knowledge_req) {
            return Err("You don't meet the knowledge requirements to convert this item".into());
        }
    }

    let is_premium_item = ctx
        .db
        .premium_item_desc()
        .iter()
        .any(|prem| prem.collectible_ids.contains(&collectible.id));
    let count = if is_premium_item { stack.quantity } else { 1 };
    stack.quantity -= count;

    inventory.set_at(request.pocket_index as usize, Some(stack));
    ctx.db.inventory_state().entity_id().update(inventory);

    let collectible_id = collectible.id;
    let mut collectibles = Vec::with_capacity(count as usize);
    for _ in 0..count {
        collectibles.push(collectible_id);
    }
    VaultState::add_collectibles(ctx, actor_id, collectibles);

    Ok(())
}
