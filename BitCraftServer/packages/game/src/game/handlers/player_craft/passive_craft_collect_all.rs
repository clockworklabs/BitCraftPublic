use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    crafting_recipe_desc,
    game::{
        entities::building_state::InventoryState,
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
    },
    messages::components::*,
    unwrap_or_continue, unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate("craft")]
pub fn passive_craft_collect_all(ctx: &ReducerContext, building_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let building_state = unwrap_or_err!(ctx.db.building_state().entity_id().find(building_entity_id), "Invalid building");

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building_state, Permission::Usage) {
        return Err("You don't have the permission to use this building".into());
    }

    // Temporary: allow a distance of 2 for when you right-click on building while moving and end up 1 tile too far by completing the current move
    if building_state.distance_to(ctx, &game_state_filters::coordinates_any(ctx, actor_id)) > 2 {
        return Err("Too far".into());
    }

    let collectable_crafts = ctx
        .db
        .passive_craft_state()
        .building_entity_id()
        .filter(building_entity_id)
        .filter(|p| p.owner_entity_id == actor_id && p.status == PassiveCraftStatus::Complete);

    let mut output = Vec::new();
    for passive_craft in collectable_crafts {
        let mut crafting_recipe_desc = unwrap_or_continue!(
            ctx.db.crafting_recipe_desc().id().find(passive_craft.recipe_id),
            "Invalid crafting recipe"
        );

        output.append(&mut crafting_recipe_desc.crafted_item_stacks);

        ctx.db.passive_craft_state().entity_id().delete(passive_craft.entity_id);
    }

    InventoryState::deposit_to_player_inventory_and_nearby_deployables(
        ctx,
        actor_id,
        &output,
        |x| building_state.distance_to(ctx, &x),
        true,
        || vec![{ game_state_filters::coordinates_any(ctx, actor_id) }],
        false,
    )?;
    player_action_helpers::post_reducer_update_cargo(ctx, actor_id);

    Ok(())
}
