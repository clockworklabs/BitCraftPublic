use std::time::Duration;

use bitcraft_macro::feature_gate;
use spacetimedb::{rand::Rng, ReducerContext, Table};

use crate::{
    game::{
        claim_helper,
        coordinates::*,
        dimensions,
        discovery::Discovery,
        entities::buff,
        game_state::{self, game_state_filters},
        reducer_helpers::player_action_helpers,
        terrain_chunk::TerrainChunkCache,
    },
    messages::{
        action_request::PlayerPlaceablePlaceRequest,
        components::*,
        game_util::{ItemType, LevelRequirement},
        static_data::{PlaceableSelfBuffChance, *},
    },
    unwrap_or_err, InventoryState,
};

const PLACEABLE_PLACEMENT_RANGE: f32 = 1.0;

fn apply_self_buffs(ctx: &ReducerContext, actor_id: u64, self_buffs: &[PlaceableSelfBuffChance]) -> Result<(), String> {
    for self_buff in self_buffs {
        if ctx.rng().gen_range(0.0..=1.0) <= self_buff.chance {
            buff::activate(ctx, actor_id, self_buff.buff_id, self_buff.duration, None)?;
        }
    }

    Ok(())
}

fn event_delay_recipe_id(
    ctx: &ReducerContext,
    request: &PlayerPlaceablePlaceRequest,
    stats: &CharacterStatsState,
) -> (Duration, Option<i32>) {
    let recipe = ctx.db.placeable_placement_desc().id().find(&request.placeable_placement_id);
    if let Some(recipe) = recipe {
        let build_speed = stats.get(CharacterStatType::BuildingSpeed).max(1.0);
        return (Duration::from_secs_f32(recipe.required_time / build_speed), Some(recipe.id));
    }

    (Duration::ZERO, None)
}

#[spacetimedb::reducer]
#[feature_gate("place")]
pub fn placeable_place_start(ctx: &ReducerContext, request: PlayerPlaceablePlaceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let stats = unwrap_or_err!(ctx.db.character_stats_state().entity_id().find(&actor_id), "Player doesn't exist");
    let target = None;
    let (delay, recipe_id) = event_delay_recipe_id(ctx, &request, &stats);

    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::PlacePlaceable,
        target,
        recipe_id,
        delay,
        reduce(ctx, actor_id, request, true),
        request.timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("place")]
pub fn placeable_place(ctx: &ReducerContext, request: PlayerPlaceablePlaceRequest) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::PlacePlaceable.get_layer(ctx),
        reduce(ctx, actor_id, request, false),
    )
}

fn reduce(ctx: &ReducerContext, actor_id: u64, request: PlayerPlaceablePlaceRequest, dry_run: bool) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;

    PlayerActionState::validate_timestamp_basic(ctx, actor_id, PlayerActionType::PlacePlaceable, request.timestamp)?;
    if !dry_run {
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::PlacePlaceable, None)?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::PlacePlaceable, request.timestamp)?;
    }

    let recipe = unwrap_or_err!(
        ctx.db.placeable_placement_desc().id().find(&request.placeable_placement_id),
        "Unknown placeable placement recipe"
    );
    let placeable_desc = unwrap_or_err!(ctx.db.placeable_desc().id().find(&recipe.placed_placeable_id), "Unknown placeable");

    let actor_coords = game_state_filters::coordinates_float(ctx, actor_id);
    let mut terrain_cache = TerrainChunkCache::empty();
    let terrain_source = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &actor_coords.parent_large_tile()),
        "Invalid location"
    );
    if terrain_source.player_should_swim() && ctx.db.mounting_state().entity_id().find(&actor_id).is_none() {
        return Err("Cannot place while swimming".into());
    }

    let coordinates = SmallHexTile::from(request.coordinates);
    // include 1 extra range as tolerance
    if actor_coords.distance_to(coordinates.into()) > PLACEABLE_PLACEMENT_RANGE + 1.0 {
        return Err("Too far".into());
    }

    validate_knowledges(ctx, actor_id, &recipe.required_knowledges, &recipe.blocking_knowledges)?;
    validate_level_requirements(ctx, actor_id, &recipe.level_requirements)?;

    if !recipe.tool_requirements.is_empty() {
        ToolDesc::get_required_tool(ctx, actor_id, &recipe.tool_requirements[0])?;
    }

    validate_location_rules(ctx, &mut terrain_cache, coordinates, &recipe, &placeable_desc)?;
    validate_other_placeable_distance_rules(ctx, coordinates, actor_id, &recipe)?;
    validate_group_rules(ctx, coordinates, actor_id, recipe.placed_placeable_id, &recipe, request, dry_run)?;
    validate_building_distance_rules(ctx, coordinates, &recipe)?;

    if !dry_run {
        let input_items = vec![recipe.input_item.clone()];
        if let Err(_) = InventoryState::withdraw_from_player_inventory_and_nearby_deployables(ctx, actor_id, &input_items, |target| {
            target.distance_to(coordinates)
        }) {
            let item_name = match recipe.input_item.item_type {
                ItemType::Item => ctx
                    .db
                    .item_desc()
                    .id()
                    .find(&recipe.input_item.item_id)
                    .map(|item| item.name)
                    .unwrap_or_else(|| "Unknown item".into()),
                ItemType::Cargo => ctx
                    .db
                    .cargo_desc()
                    .id()
                    .find(&recipe.input_item.item_id)
                    .map(|cargo| cargo.name)
                    .unwrap_or_else(|| "Unknown cargo".into()),
            };

            return Err(format!("Requires: {{0}} x{{1}}|~{}|~{}", item_name, recipe.input_item.quantity));
        }

        PlaceableState::spawn(ctx, placeable_desc.id, actor_id, coordinates, request.facing_direction)?;
        if let Some(self_buffs) = &recipe.self_buffs {
            apply_self_buffs(ctx, actor_id, self_buffs)?;
        }
    }

    Ok(())
}

fn validate_level_requirements(ctx: &ReducerContext, actor_id: u64, level_requirements: &[LevelRequirement]) -> Result<(), String> {
    if level_requirements.is_empty() {
        return Ok(());
    }

    let experience = unwrap_or_err!(
        ctx.db.experience_state().entity_id().find(&actor_id),
        "Player has no experience state"
    );
    for requirement in level_requirements {
        if experience.get_level(requirement.skill_id) < requirement.level {
            return Err("You don't meet the level requirements to place this".into());
        }
    }

    Ok(())
}

fn validate_knowledges(
    ctx: &ReducerContext,
    actor_id: u64,
    required_knowledges: &[i32],
    blocking_knowledges: &[i32],
) -> Result<(), String> {
    for required_knowledge_id in required_knowledges {
        if !Discovery::already_acquired_secondary(ctx, actor_id, *required_knowledge_id) {
            return Err("You don't have the knowledge required to place this".into());
        }
    }

    if !blocking_knowledges.is_empty() {
        let mut possess_all_knowledges = true;
        let secondary_knowledge = unwrap_or_err!(
            ctx.db.knowledge_secondary_state().entity_id().find(actor_id),
            "Player missing knowledge state"
        );
        for knowledge_id in blocking_knowledges {
            possess_all_knowledges &= secondary_knowledge
                .entries
                .iter()
                .any(|knowledge| knowledge.id == *knowledge_id && knowledge.state == KnowledgeState::Acquired);
        }
        if possess_all_knowledges {
            return Err("You don't need to place this anymore".into());
        }
    }

    Ok(())
}

fn validate_location_rules(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    coordinates: SmallHexTile,
    recipe: &PlaceablePlacementDesc,
    placeable_desc: &PlaceableDesc,
) -> Result<(), String> {
    let terrain = unwrap_or_err!(
        terrain_cache.get_terrain_cell(ctx, &coordinates.parent_large_tile()),
        "Invalid location"
    );

    if !recipe.required_biomes.is_empty() && recipe.required_biomes.iter().all(|biome| terrain.biome_percentage(*biome) == 0.0) {
        return Err("Can't be placed in this biome".into());
    }

    PlaceableState::validate_spawn_terrain(terrain_cache, ctx, coordinates, placeable_desc)?;

    if recipe.required_paving_tier >= 0 {
        let valid = match PavedTileState::get_at_location(ctx, &coordinates) {
            Some(tile) => ctx.db.paving_tile_desc().id().find(&tile.tile_type_id).unwrap().tier >= recipe.required_paving_tier,
            None => false,
        };
        if !valid {
            if recipe.required_paving_tier == 0 {
                return Err("This placeable requires paving!".into());
            }
            return Err(format!("This placeable requires tier {{0}} paving!|~{}", recipe.required_paving_tier));
        }
    }

    let dimension = unwrap_or_err!(
        ctx.db.dimension_description_state().dimension_id().find(&coordinates.dimension),
        "Invalid dimension"
    );
    if recipe.required_interior_tier == -1 && dimension.interior_instance_id != 0 {
        return Err("Can only be built in Overworld".into());
    }
    if recipe.required_interior_tier > 0 {
        if dimension.interior_instance_id == 0 {
            return Err(format!("Requires Tier {{0}} interior|~{}", recipe.required_interior_tier));
        }

        let interior = ctx.db.interior_instance_desc().id().find(&dimension.interior_instance_id).unwrap();
        if interior.tier < recipe.required_interior_tier {
            return Err(format!("Requires Tier {{0}} interior|~{}", recipe.required_interior_tier));
        }
    }

    if recipe.required_claim_tier > 0 {
        let claim = unwrap_or_err!(
            claim_helper::get_claim_on_tile(ctx, coordinates),
            "This placeable needs to be placed on a claim"
        );
        let claim_tech = unwrap_or_err!(
            ctx.db.claim_tech_state().entity_id().find(&claim.claim_id),
            "This claim is missing its tech tree"
        );
        if claim_tech.max_tier(ctx) < recipe.required_claim_tier {
            return Err(format!("Requires Tier {{0}} claim|~{}", recipe.required_claim_tier));
        }
    }

    if coordinates.dimension == dimensions::OVERWORLD
        && recipe.min_distance_to_player_claims > 0
        && game_state_filters::any_claims_in_radius(ctx, coordinates, recipe.min_distance_to_player_claims - 1)
    {
        return Err("Too close to another claim".into());
    }

    if recipe.min_distance_to_existing_footprints > 0
        && has_footprint_in_radius(ctx, coordinates, recipe.min_distance_to_existing_footprints - 1)
    {
        return Err("Too close to another structure".into());
    }

    Ok(())
}

fn validate_group_rules(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    actor_id: u64,
    placeable_id: i32,
    recipe: &PlaceablePlacementDesc,
    request: PlayerPlaceablePlaceRequest,
    dry_run: bool,
) -> Result<(), String> {
    let matching_groups: Vec<PlaceableGroupDesc> = ctx
        .db
        .placeable_group_desc()
        .iter()
        .filter(|group| group.placeable_ids.contains(&placeable_id))
        .collect();

    for group in matching_groups {
        let owned_group_placeables: Vec<PlaceableState> = ctx
            .db
            .placeable_state()
            .owner_entity_id()
            .filter(actor_id)
            .filter(|placeable| group.placeable_ids.contains(&placeable.placeable_id))
            .collect();

        if owned_group_placeables.len() >= group.placement_limit as usize {
            if !request.replace_oldest_in_full_group {
                return Err(format!("You can only place {{0}} {{1}}|~{}|~{}", group.placement_limit, group.name));
            }

            if !dry_run {
                let oldest_group_placeable = owned_group_placeables.iter().min_by_key(|placeable| placeable.entity_id).unwrap();
                oldest_group_placeable.despawn(ctx);
            }
        }

        if recipe.min_distance_to_group > 0
            && owned_group_placeables
                .iter()
                .any(|placeable| placeable.distance_to(ctx, coordinates) < recipe.min_distance_to_group)
        {
            return Err("Too close to another one of your placeables".into());
        }
    }

    Ok(())
}

fn validate_other_placeable_distance_rules(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    actor_id: u64,
    recipe: &PlaceablePlacementDesc,
) -> Result<(), String> {
    if recipe.min_distance_to_other_placeables <= 0 {
        return Ok(());
    }

    let owned_placeables_too_close = ctx
        .db
        .placeable_state()
        .owner_entity_id()
        .filter(actor_id)
        .any(|placeable| placeable.distance_to(ctx, coordinates) < recipe.min_distance_to_other_placeables);

    if owned_placeables_too_close {
        return Err("Too close to another one of your placeables".into());
    }

    Ok(())
}

fn validate_building_distance_rules(
    ctx: &ReducerContext,
    coordinates: SmallHexTile,
    recipe: &PlaceablePlacementDesc,
) -> Result<(), String> {
    if recipe.buildings.is_empty() || recipe.max_distance_to_buildings <= 0 {
        return Ok(());
    }

    let has_matching_building_in_range = ctx.db.building_state().iter().any(|building| {
        if !recipe.buildings.contains(&building.building_description_id) {
            return false;
        }

        let building_coordinates = game_state_filters::coordinates(ctx, building.entity_id);
        building_coordinates.dimension == coordinates.dimension
            && building_coordinates.distance_to(coordinates) <= recipe.max_distance_to_buildings
    });

    if !has_matching_building_in_range {
        return Err("Must be placed near one of the required buildings".into());
    }

    Ok(())
}

fn has_footprint_in_radius(ctx: &ReducerContext, coordinates: SmallHexTile, radius: i32) -> bool {
    SmallHexTile::coordinates_in_radius_with_center_iter(coordinates, radius)
        .any(|coord| FootprintTileState::get_at_location(ctx, &coord).next().is_some())
}
