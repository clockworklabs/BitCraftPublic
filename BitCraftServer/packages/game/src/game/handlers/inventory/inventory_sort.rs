use bitcraft_macro::feature_gate;
use std::cmp::Ordering;

use crate::game::game_state;
use crate::messages::components::*;
use crate::{cargo_desc, item_desc, unwrap_or_err};
use spacetimedb::ReducerContext;

use super::inventory_helper;

#[spacetimedb::reducer]
#[feature_gate]
pub fn inventory_sort(ctx: &ReducerContext, target_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let mut target_inventory = unwrap_or_err!(
        ctx.db.inventory_state().entity_id().find(target_entity_id),
        "Invalid target inventory"
    );

    let player_location = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(actor_id), "Player has no location").coordinates();

    inventory_helper::validate_interact(
        ctx,
        actor_id,
        player_location,
        target_inventory.owner_entity_id,
        target_inventory.player_owner_entity_id,
    )?;

    let mut pockets_copy = target_inventory.pockets.clone();
    let length = pockets_copy.len();

    for i in 0..length - 1 {
        let pocket = &pockets_copy[i];
        if pocket.locked {
            return Err("This inventory contains locked pockets and can't be sorted.".into());
        }

        if pocket.contents.is_none() {
            continue;
        }

        let is_cargo = target_inventory.is_pocket_cargo(i);

        let stack = pocket.contents.unwrap();
        let item_id = stack.item_id;

        let item_volume = if is_cargo {
            let cargo_def = ctx.db.cargo_desc().id().find(item_id).unwrap();
            cargo_def.volume
        } else {
            let item_def = ctx.db.item_desc().id().find(item_id).unwrap();
            item_def.volume
        };

        let mut available_space = pocket.can_fit_quantity(ctx, item_volume, is_cargo);

        if available_space <= 0 {
            continue;
        }

        let mut pocket_copy = pocket.clone();

        for (j, other_pocket) in pockets_copy.iter_mut().enumerate().skip(i + 1) {
            if other_pocket.locked {
                return Err("This inventory contains locked pockets and can't be sorted.".into());
            }
            if is_cargo != target_inventory.is_pocket_cargo(j) || other_pocket.contents.is_none() {
                continue;
            }

            let other_stack = other_pocket.contents.unwrap();
            let other_item_id = other_stack.item_id;

            if item_id != other_item_id {
                continue;
            }

            let mut other_pocket_copy = other_pocket.clone();

            let moving_quantity = other_pocket_copy.remove_quantity(available_space);
            pocket_copy.add_quantity(moving_quantity);

            *other_pocket = other_pocket_copy;

            available_space -= moving_quantity;
            if available_space <= 0 {
                break;
            }
        }

        pockets_copy[i] = pocket_copy;
    }

    let cargo_index = target_inventory.cargo_index as usize;

    pockets_copy[0..cargo_index].sort_by(|a, b| -> Ordering {
        if a.contents.is_none() {
            return Ordering::Greater;
        }
        if b.contents.is_none() {
            return Ordering::Less;
        }

        let a_stack = a.contents.unwrap();
        let b_stack = b.contents.unwrap();

        let a_def = ctx.db.item_desc().id().find(a_stack.item_id).unwrap();
        let b_def = ctx.db.item_desc().id().find(b_stack.item_id).unwrap();

        a_def
            .tag
            .cmp(&b_def.tag)
            .then_with(|| b_def.tier.cmp(&a_def.tier))
            .then_with(|| b_def.rarity.cmp(&a_def.rarity))
            .then_with(|| a_def.id.cmp(&b_def.id))
            .then_with(|| b_stack.quantity.cmp(&a_stack.quantity))
    });

    pockets_copy[cargo_index..length].sort_by(|a, b| -> Ordering {
        if a.contents.is_none() {
            return Ordering::Greater;
        }
        if b.contents.is_none() {
            return Ordering::Less;
        }

        let a_stack = a.contents.unwrap();
        let b_stack = b.contents.unwrap();

        let a_def = ctx.db.cargo_desc().id().find(a_stack.item_id).unwrap();
        let b_def = ctx.db.cargo_desc().id().find(b_stack.item_id).unwrap();

        a_def
            .tag
            .cmp(&b_def.tag)
            .then_with(|| b_def.tier.cmp(&a_def.tier))
            .then_with(|| b_def.rarity.cmp(&a_def.rarity))
            .then_with(|| a_def.id.cmp(&b_def.id))
            .then_with(|| b_stack.quantity.cmp(&a_stack.quantity))
    });

    target_inventory.pockets = pockets_copy;

    ctx.db.inventory_state().entity_id().update(target_inventory);

    Ok(())
}
