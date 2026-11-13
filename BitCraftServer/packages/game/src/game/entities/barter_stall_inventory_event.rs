use spacetimedb::{ReducerContext, Table};

use crate::messages::{
    components::*,
    events::{barter_stall_inventory_event, BarterStallInventoryChangeReason, BarterStallInventoryEvent},
    game_util::ItemStack,
};

/// An inventory row before and after a reducer modified it.
pub type InventoryChange<'a> = (&'a InventoryState, &'a InventoryState);

impl BarterStallInventoryEvent {
    /// Emits a Sale event for a trade order accepted `trade_amount` times.
    /// `changes` are the stall's own inventories.
    #[expect(clippy::too_many_arguments, reason = "pre-existing signature, left as is when clippy was enabled")]
    pub fn emit_sale<'a>(
        ctx: &ReducerContext,
        actor_entity_id: u64,
        shop_entity_id: u64,
        trade_order_entity_id: u64,
        trade_amount: i32,
        changes: impl IntoIterator<Item = InventoryChange<'a>>,
        treasury_coins_added: i32,
        treasury_coins_removed: i32,
    ) {
        let Some(claim_entity_id) = barter_stall_claim_entity_id(ctx, shop_entity_id) else {
            return;
        };
        let changes: Vec<InventoryChange> = changes.into_iter().collect();
        let (added_items, removed_items) = net_item_change(&changes);
        ctx.db.barter_stall_inventory_event().insert(BarterStallInventoryEvent {
            shop_entity_id,
            claim_entity_id,
            actor_entity_id,
            reason: BarterStallInventoryChangeReason::Sale,
            trade_order_entity_id: Some(trade_order_entity_id),
            trade_amount: Some(trade_amount),
            added_items,
            removed_items,
            treasury_coins_added,
            treasury_coins_removed,
            timestamp: ctx.timestamp,
        });
    }
}

/// Returns the stall's claim_entity_id (0 when unclaimed), or None if the entity isn't a barter stall building.
fn barter_stall_claim_entity_id(ctx: &ReducerContext, entity_id: u64) -> Option<u64> {
    ctx.db.barter_stall_state().entity_id().find(entity_id)?;
    ctx.db.building_state().entity_id().find(entity_id).map(|b| b.claim_entity_id)
}

/// Net quantity change per (item_id, item_type) across `changes`, split into (added, removed).
/// Both lists hold positive quantities. Durability is ignored.
fn net_item_change(changes: &[InventoryChange]) -> (Vec<ItemStack>, Vec<ItemStack>) {
    let signed_stacks: Vec<ItemStack> = changes
        .iter()
        .flat_map(|(before, after)| {
            let removed = before.as_item_stacks().into_iter().map(|s| s.clone_with_quantity(-s.quantity));
            removed.chain(after.as_item_stacks())
        })
        .collect();

    let (added, removed): (Vec<ItemStack>, Vec<ItemStack>) = ItemStack::merge_multiple(&signed_stacks)
        .into_iter()
        .filter(|s| s.quantity != 0)
        .map(|s| ItemStack::new_ignore_durability(s.item_id, s.item_type, s.quantity))
        .partition(|s| s.quantity > 0);
    let removed = removed.into_iter().map(|s| s.clone_with_quantity(-s.quantity)).collect();

    (added, removed)
}
