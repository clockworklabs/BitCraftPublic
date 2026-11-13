use spacetimedb::{ReducerContext, Table};

use crate::{
    game::game_state,
    messages::{components::*, game_util::ItemStack},
    unwrap_or_return,
};

use super::inventory_type::InventoryType;

impl ActionLogState {
    pub fn log_storage(
        ctx: &ReducerContext,
        from_inventory: &InventoryState,
        to_inventory: &InventoryState,
        player_entity_id: u64,
        inventory_type_from: InventoryType,
        inventory_type_to: InventoryType,
        item: &ItemStack,
    ) {
        let player_name = unwrap_or_return!(
            ctx.db.player_username_state().entity_id().find(player_entity_id),
            "Player not found"
        )
        .username;

        let from_entity_id = from_inventory.owner_entity_id;
        let to_entity_id = to_inventory.owner_entity_id;

        if Self::should_log(from_inventory, inventory_type_from) {
            ctx.db.storage_log_state().insert(ActionLogState {
                id: 0, //autoinc
                object_entity_id: from_entity_id,
                subject_entity_id: player_entity_id,
                subject_name: player_name.clone(),
                subject_type: ActionLogSubjectType::Player,
                data: ActionLogData::WithdrawItem(*item),
                timestamp: ctx.timestamp,
                days_since_epoch: game_state::days_since_unix_epoch(ctx.timestamp),
            });
        }
        if Self::should_log(to_inventory, inventory_type_to) {
            ctx.db.storage_log_state().insert(ActionLogState {
                id: 0, //autoinc
                object_entity_id: to_entity_id,
                subject_entity_id: player_entity_id,
                subject_name: player_name,
                subject_type: ActionLogSubjectType::Player,
                data: ActionLogData::DepositItem(*item),
                timestamp: ctx.timestamp,
                days_since_epoch: game_state::days_since_unix_epoch(ctx.timestamp),
            });
        }
    }

    fn should_log(inventory: &InventoryState, inventory_type: InventoryType) -> bool {
        inventory_type == InventoryType::Building && inventory.player_owner_entity_id == 0
    }
}
