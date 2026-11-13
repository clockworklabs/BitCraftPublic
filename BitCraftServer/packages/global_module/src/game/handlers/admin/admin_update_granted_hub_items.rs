use crate::{
    game::handlers::authentication::has_role,
    inter_module::grant_hub_item,
    messages::{
        authentication::Role,
        components::user_state,
        generic::HubItemType,
        global::{granted_hub_item_state, player_shard_state, user_region_state, GrantedHubItemState},
    },
    unwrap_or_err,
};
use spacetimedb::{log, Identity, ReducerContext, Table};

#[spacetimedb::reducer]
pub fn admin_update_granted_hub_item_state(
    ctx: &ReducerContext,
    identity: Identity,
    item_type: HubItemType,
    item_id: i32,
    balance: u32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut granted_hub_item_state_entity_id = 0;
    let mut delta = balance;

    if let Some(mut existing) = ctx
        .db
        .granted_hub_item_state()
        .identity_and_item_id()
        .filter((identity, item_id))
        .find(|x| x.item_type == item_type)
    {
        granted_hub_item_state_entity_id = existing.entity_id;
        if let Some(new_delta) = delta.checked_sub(existing.balance) {
            delta = new_delta;
        } else {
            log::error!(
                "existing granted balance {} is greater than hub balance {}. existing.entity_id: {}",
                existing.balance,
                balance,
                existing.entity_id
            );
            existing.balance = balance;
            ctx.db.granted_hub_item_state().entity_id().update(existing);
            return Ok(());
        }
    }

    if delta == 0 {
        return Ok(());
    }

    if item_type == HubItemType::HexiteShards {
        return update_granted_shards(ctx, granted_hub_item_state_entity_id, identity, balance, delta);
    }

    update_granted_hub_item_state(ctx, granted_hub_item_state_entity_id, identity, item_type, item_id, balance, delta)
}

fn update_granted_shards(
    ctx: &ReducerContext,
    granted_hub_item_state_entity_id: u64,
    identity: Identity,
    balance: u32,
    delta: u32,
) -> Result<(), String> {
    let user_state = unwrap_or_err!(ctx.db.user_state().identity().find(identity), "Unknown user");
    let mut player_shard_state = unwrap_or_err!(
        ctx.db.player_shard_state().entity_id().find(user_state.entity_id),
        "Unknown player shard state"
    );

    player_shard_state.shards += delta;
    ctx.db.player_shard_state().entity_id().update(player_shard_state);

    insert_or_update_granted_hub_item(
        ctx,
        granted_hub_item_state_entity_id,
        identity,
        HubItemType::HexiteShards,
        0,
        balance,
    )
}

fn update_granted_hub_item_state(
    ctx: &ReducerContext,
    granted_hub_item_state_entity_id: u64,
    identity: Identity,
    item_type: HubItemType,
    item_id: i32,
    balance: u32,
    delta: u32,
) -> Result<(), String> {
    let user_region_state = unwrap_or_err!(ctx.db.user_region_state().identity().find(identity), "Unknown user_region_state");

    grant_hub_item::send_message(ctx, identity, item_type, item_id, delta, user_region_state.region_id)?;
    insert_or_update_granted_hub_item(ctx, granted_hub_item_state_entity_id, identity, item_type, item_id, balance)
}

fn insert_or_update_granted_hub_item(
    ctx: &ReducerContext,
    entity_id: u64,
    identity: Identity,
    item_type: HubItemType,
    item_id: i32,
    balance: u32,
) -> Result<(), String> {
    if let Some(mut existing) = ctx.db.granted_hub_item_state().entity_id().find(entity_id) {
        existing.balance = balance;
        ctx.db.granted_hub_item_state().entity_id().update(existing);
        return Ok(());
    }

    if ctx
        .db
        .granted_hub_item_state()
        .try_insert(GrantedHubItemState {
            entity_id,
            identity,
            item_type,
            item_id,
            balance,
        })
        .is_err()
    {
        return Err("Failed to insert GrantedHubItemState".into());
    }

    Ok(())
}
