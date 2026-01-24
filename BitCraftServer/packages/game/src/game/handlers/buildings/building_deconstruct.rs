use std::time::Duration;

use crate::{
    building_desc, deconstruction_recipe_desc,
    game::{
        dimensions,
        discovery::Discovery,
        entities::building_state::InventoryState,
        game_state::{self, game_state_filters},
        handlers::player::player_use_elevator::player_use_elevator_timer,
        permission_helper,
        reducer_helpers::{building_helpers::delete_building, player_action_helpers},
    },
    inter_module::send_inter_module_message,
    messages::{
        action_request::PlayerBuildingDeconstructRequest,
        components::*,
        empire_shared::{empire_node_siege_state, EmpirePermission, EmpirePlayerDataState},
        game_util::{DimensionType, ItemType},
        inter_module::{GlobalDeleteEmpireBuildingMsg, MessageContentsV4},
        static_data::{DeconstructionRecipeDesc, ToolDesc},
    },
    parameters_desc_v2, unwrap_or_err, unwrap_or_return, BuildingCategory, ItemListDesc,
};
use bitcraft_macro::shared_table_reducer;
use spacetimedb::ReducerContext;

pub fn event_delay_recipe_id(
    ctx: &ReducerContext,
    actor_id: u64,
    request: &PlayerBuildingDeconstructRequest,
    stats: CharacterStatsState,
) -> (Duration, Option<i32>) {
    if let Some(building) = ctx.db.building_state().entity_id().find(&request.building_entity_id) {
        if let Some(recipe) = get_recipe(ctx, building.building_description_id) {
            let mut delay = recipe.time_requirement;
            // Tech time reduction
            let skill_speed = match recipe.get_skill_type() {
                Some(skill) => stats.get_skill_speed(skill),
                None => 1.0,
            };
            if recipe.tool_requirements.len() > 0 {
                let desired_tool_power = recipe.tool_requirements[0].power;
                let tool = ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0]);
                let time_factor = match tool {
                    Ok(t) => ToolDesc::get_time_factor(ctx, t.power, desired_tool_power),
                    Err(_) => return (Duration::ZERO, None), // React will invalidate it.
                };
                delay *= time_factor;
            }
            return (Duration::from_secs_f32(delay / skill_speed), Some(recipe.id));
        } else {
            let default_delay = ctx.db.parameters_desc_v2().version().find(&0).unwrap().deconstruct_default_time;
            return (Duration::from_secs_f32(default_delay), None);
        }
    }
    (Duration::ZERO, None)
}

#[spacetimedb::reducer]
pub fn building_deconstruct_start(ctx: &ReducerContext, request: PlayerBuildingDeconstructRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    let target = Some(request.building_entity_id);
    let (delay, recipe_id) = event_delay_recipe_id(ctx, actor_id, &request, stats);
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Deconstruct,
        target,
        recipe_id,
        delay,
        reduce(ctx, actor_id, &request, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn building_deconstruct(ctx: &ReducerContext, request: PlayerBuildingDeconstructRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::Deconstruct.get_layer(ctx),
        reduce(ctx, actor_id, &request, false),
    )
}

pub fn reduce(ctx: &ReducerContext, actor_id: u64, request: &PlayerBuildingDeconstructRequest, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        // Make sure target and timestamp and action fit
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Deconstruct, Some(request.building_entity_id))?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Deconstruct, request.timestamp)?;
    }

    let building_entity_id = request.building_entity_id;
    let building = unwrap_or_err!(
        ctx.db.building_state().entity_id().find(&building_entity_id),
        "Could not find building"
    );

    HealthState::check_incapacitated(ctx, actor_id, true)?;

    let coord = game_state_filters::coordinates(ctx, building_entity_id);

    let building_desc = ctx.db.building_desc().id().find(&building.building_description_id).unwrap();
    if building_desc.not_deconstructible {
        return Err("That building cannot be deconstructed.".into());
    }
    if building_desc.functions.iter().find(|f| f.trade_orders > 0).is_some() {
        if !PermissionState::can_interact_with_tile(ctx, actor_id, coord, Permission::Build) {
            return Err("You don't have permission to interact with this building".into());
        }
        // This is a barter stall. For deconstruct don't use can_interact_with_building as stalls are set to interaction level All
        if !permission_helper::can_interact_with_tile(ctx, coord, actor_id, ClaimPermission::Build) {
            return Err("You don't have permission to interact with this building".into());
        }
    }

    if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    if building_desc.has_category(ctx, BuildingCategory::PremiumBuilding) {
        if let Some(claim) = ctx.db.claim_state().entity_id().find(building.claim_entity_id) {
            if building_desc.has_category(ctx, BuildingCategory::PremiumBuilding) && !claim.has_co_owner_permissions(ctx, actor_id) {
                return Err("Premium buildings can only be deconstructed by the claim owner or co-owners".into());
            }
        }
        if !PermissionState::can_interact_with_building(ctx, actor_id, &building, Permission::CoOwner) {
            return Err("Premium buildings can only be deconstructed by owner or co-owners".into());
        }
    }

    if !permission_helper::can_interact_with_building(ctx, &building, actor_id, ClaimPermission::Build) {
        return Err("You don't have permission to interact with this building".into());
    }

    if coord.dimension != dimensions::OVERWORLD {
        if let Some(_) = ctx.db.portal_state().entity_id().find(&building_entity_id) {
            return Err("Can't deconstruct portal buildings inside interiors".into());
        }
        let dimension_desc = unwrap_or_err!(
            ctx.db.dimension_description_state().dimension_id().find(&coord.dimension),
            "Invalid coordinates"
        );
        if dimension_desc.dimension_type == DimensionType::AncientRuin || dimension_desc.dimension_type == DimensionType::Dungeon {
            return Err("Can't deconstruct buildings inside ancient ruins".into());
        }
    }

    if building.distance_to(ctx, &game_state_filters::coordinates_float(ctx, actor_id).into()) > 2 {
        return Err("Too far".into());
    }

    if building_desc.has_category(ctx, BuildingCategory::PlayerHousing) {
        for housing in ctx
            .db
            .player_housing_state()
            .entrance_building_entity_id()
            .filter(building_entity_id)
        {
            if housing.locked_until <= ctx.timestamp {
                return Err("You can't deconstruct building that has residents".into());
            }
        }
    }

    let recipe = get_recipe(ctx, building.building_description_id);
    if let Some(recipe) = &recipe {
        // validate tool requirements
        if recipe.tool_requirements.len() > 0 {
            if let Err(err_str) = ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0]) {
                return Err(err_str.into());
            }
        }
    }

    if !dry_run {
        if building_desc.has_category(ctx, BuildingCategory::Watchtower) {
            if ctx
                .db
                .empire_node_siege_state()
                .building_entity_id()
                .filter(building_entity_id)
                .filter(|node| node.active)
                .count()
                > 0
            {
                return Err("Cannot deconstruct a watchtower under siege".into());
            }

            // Make sure player has MarkForExpansion permission
            if !EmpirePlayerDataState::has_permission(ctx, actor_id, EmpirePermission::MarkAreaForExpansion) {
                return Err("You don't have the permission to deconstruct a watchtower".into());
            }
            send_inter_module_message(
                ctx,
                MessageContentsV4::GlobalDeleteEmpireBuilding(GlobalDeleteEmpireBuildingMsg {
                    player_entity_id: actor_id,
                    building_entity_id,
                }),
                crate::inter_module::InterModuleDestination::Global,
            );
        }

        if let Some(recipe) = recipe {
            grant_deconstructed_items(ctx, actor_id, &recipe);
        }
        // dont' drop items for bank inventories, these will go into the recovery chest
        let drop_inventory_items = !building_desc.has_category(ctx, BuildingCategory::Bank);
        delete_building(ctx, actor_id, building_entity_id, None, false, drop_inventory_items);

        if building_desc.has_category(ctx, BuildingCategory::Elevator) {
            if ctx
                .db
                .player_use_elevator_timer()
                .origin_platform_entity_id()
                .find(&building_entity_id)
                .is_some()
                || ctx
                    .db
                    .player_use_elevator_timer()
                    .destination_platform_entity_id()
                    .find(&building_entity_id)
                    .is_some()
            {
                return Err("Cannot deconstruct an elevator that is in use".into());
            }
        }

        player_action_helpers::post_reducer_update_cargo(ctx, actor_id);
    }

    ctx.db.waystone_state().building_entity_id().delete(building_entity_id);
    ctx.db.bank_state().building_entity_id().delete(building_entity_id);
    ctx.db.marketplace_state().building_entity_id().delete(building_entity_id);

    Ok(())
}

fn get_recipe(ctx: &ReducerContext, building_description_id: i32) -> Option<DeconstructionRecipeDesc> {
    ctx.db
        .deconstruction_recipe_desc()
        .consumed_building()
        .filter(building_description_id)
        .next()
}

pub fn grant_deconstructed_items_for_entity(ctx: &ReducerContext, actor_id: u64, building_entity_id: u64) {
    let building = ctx.db.building_state().entity_id().find(&building_entity_id).unwrap();
    if let Some(recipe) = get_recipe(ctx, building.building_description_id) {
        grant_deconstructed_items(ctx, actor_id, &recipe);
    }
}

pub fn grant_deconstructed_items(ctx: &ReducerContext, actor_id: u64, recipe: &DeconstructionRecipeDesc) {
    // update inventory
    let mut discovery = Discovery::new(actor_id);

    let mut inventory = unwrap_or_return!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    for stack in &recipe.output_item_stacks {
        if stack.item_type == ItemType::Cargo {
            inventory.add_multiple_with_overflow(ctx, &vec![stack.clone()]);
            continue;
        }

        let mut converted_stacks = ItemListDesc::extract_item_stacks_from_item(ctx, stack.clone());
        for output in converted_stacks.iter_mut() {
            discovery.acquire_item_stack(ctx, output);
            output.auto_collect(ctx, &mut discovery, actor_id);
        }
        inventory.add_multiple_with_overflow(ctx, &converted_stacks);
    }

    discovery.commit(ctx);

    ctx.db.inventory_state().entity_id().update(inventory);

    if let Some(experience_per_progress) = recipe.experience_per_progress.get(0) {
        ExperienceState::add_experience(
            ctx,
            actor_id,
            experience_per_progress.skill_id,
            f32::ceil(experience_per_progress.quantity) as i32,
        );
    }
}
