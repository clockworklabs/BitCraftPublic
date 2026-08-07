use bitcraft_macro::feature_gate;
use crate::game::reducer_helpers::player_action_helpers;
use crate::game::terrain_chunk::TerrainChunkCache;
use crate::messages::components::PlayerActionState;
use crate::messages::game_util::ItemStack;
use crate::messages::util::SmallHexTileMessage;
use crate::{
    game::{entities::building_state::InventoryState, game_state, reducer_helpers::player_action_helpers::post_reducer_update_cargo},
    messages::components::*,
    messages::static_data::*,
    unwrap_or_err,
};
use spacetimedb::{ReducerContext, Table};
use std::time::Duration;

const LOGGING: bool = false;

#[macro_export]
macro_rules! log {
    ($($arg:tt)+) => (if LOGGING { spacetimedb::log::info!($($arg)+); })
}

#[spacetimedb::reducer]
#[feature_gate("prospecting")]
pub fn prospect_start(ctx: &ReducerContext, prospecting_id: i32, timestamp: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let target = None;
    let delay = event_delay(ctx, prospecting_id);
    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::start_action(
        ctx,
        actor_id,
        PlayerActionType::Prospect,
        target,
        None,
        delay,
        self::reduce(ctx, &mut terrain_cache, actor_id, prospecting_id, timestamp, true),
        timestamp,
    )
}

#[spacetimedb::reducer]
#[feature_gate("prospecting")]
pub fn prospect(ctx: &ReducerContext, prospecting_id: i32, timestamp: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(&ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);
    let mut terrain_cache = TerrainChunkCache::empty();
    player_action_helpers::schedule_clear_player_action(
        actor_id,
        PlayerActionType::PaveTile.get_layer(ctx),
        self::reduce(ctx, &mut terrain_cache, actor_id, prospecting_id, timestamp, false),
    )
}

fn event_delay(ctx: &ReducerContext, prospecting_id: i32) -> Duration {
    let prospecting = ctx.db.prospecting_desc().id().find(prospecting_id);
    if prospecting.is_none() {
        return Duration::ZERO;
    }
    Duration::from_secs_f32(prospecting.unwrap().prospecting_duration)
}

fn reveal_reward_for_zero_crumb_trail(
    ctx: &ReducerContext,
    crumb_trail: &mut CrumbTrailState,
    prospecting_desc: &ProspectingDesc,
) {
    let mut exposed = ctx
        .db
        .crumb_trail_exposed_state()
        .crumb_trail_entity_id()
        .find(crumb_trail.entity_id)
        .unwrap();

    exposed.exposed_locations.push(crumb_trail.prize_location);
    if prospecting_desc.enemy_ai_desc_id != 0 {
        exposed.exposed_herd_entity_id = crumb_trail.spawn_herd_prize(ctx, prospecting_desc);
    } else {
        crumb_trail.replace_prize_resources(ctx, prospecting_desc);
    }

    ctx.db.crumb_trail_exposed_state().crumb_trail_entity_id().update(exposed);
}

fn contribution_for_uncovered_breadcrumb(prospecting_desc: &ProspectingDesc, current_contribution: i32) -> i32 {
    let mut contribution = current_contribution + prospecting_desc.contribution_per_visited_bread_crumb;
    if prospecting_desc.single_contribution_only {
        contribution = contribution.min(1);
    }
    contribution
}

fn reduce(
    ctx: &ReducerContext,
    terrain_cache: &mut TerrainChunkCache,
    actor_id: u64,
    prospecting_id: i32,
    timestamp: u64,
    dry_run: bool,
) -> Result<(), String> {
    HealthState::check_incapacitated(ctx, actor_id, true)?;
    if !dry_run {
        // Make sure target and timestamp and action fitP
        PlayerActionState::validate(ctx, actor_id, PlayerActionType::Prospect, None)?;
        PlayerActionState::validate_action_timing(ctx, actor_id, PlayerActionType::Prospect, timestamp)?;
    }

    // Validate if ability is locked
    AbilityUnlockDesc::evaluate(
        ctx,
        actor_id,
        AbilityTypeEnum::Prospecting,
        AbilityType::Prospecting(prospecting_id),
    )?;

    let prospecting_desc = unwrap_or_err!(ctx.db.prospecting_desc().id().find(prospecting_id), "Invalid prospecting entry");

    // Verify distance to paving target
    let location = unwrap_or_err!(ctx.db.mobile_entity_state().entity_id().find(&actor_id), "Invalid player");
    let player_coord = location.coordinates();

    let terrain = terrain_cache.get_terrain_cell(ctx, &player_coord.parent_large_tile()).unwrap();

    // If a player isn't on a deployable and is in swim-deep water they aren't allowed to prospect
    if terrain.player_should_swim() && ctx.db.mounting_state().entity_id().find(actor_id).is_none() {
        return Err("Action disallowed while swimming".into());
    }

    let mut player_prospecting = ctx.db.prospecting_state().entity_id().find(actor_id);

    // Destroy previous prospecting if we start a different kind. Client will show a confirmation pop-up so it doesn't happen accidentally.
    if player_prospecting.is_some() && player_prospecting.as_ref().unwrap().prospecting_id != prospecting_desc.id {
        ctx.db.prospecting_state().entity_id().delete(player_prospecting.unwrap().entity_id);
        player_prospecting = None;
    }

    // Initial Prospecting means we can join or create a trail
    let is_initial_prospecting = player_prospecting.is_none();

    if terrain.is_submerged() {
        if !prospecting_desc.allow_aquatic_bread_crumb && is_initial_prospecting {
            return Err("You cannot start prospecting in water".into());
        }
        if !prospecting_desc.allow_aquatic_prospecting && !is_initial_prospecting {
            return Err("You cannot do that prospecting in water".into());
        }
    }

    if !prospecting_desc.biome_requirements.contains(&terrain.biome()) {
        return Err("You cannot prospect in that biome".into());
    }

    let mut item_inventory = unwrap_or_err!(InventoryState::get_player_inventory(ctx, actor_id), "Player has no inventory");

    let consumed_item_stacks: Vec<ItemStack> = prospecting_desc
        .consumed_items_by_ability_trigger
        .iter()
        .map(|i| ItemStack::new(ctx, i.item_id, i.item_type, i.quantity))
        .collect();

    if consumed_item_stacks.len() > 0 {
        if dry_run {
            if !item_inventory.has(&consumed_item_stacks) {
                return Err("You don't have the required items.".into());
            }
        } else {
            if !item_inventory.remove(&consumed_item_stacks) {
                return Err("You don't have the required items.".into());
            }
            ctx.db.inventory_state().entity_id().update(item_inventory);
            post_reducer_update_cargo(ctx, actor_id);
        }
    }

    if dry_run {
        return Ok(());
    }

    let player_location = location.to_location_state().coordinates();

    log!("**********************************************");
    log!("*");
    if is_initial_prospecting {
        log!("* INITIATING NEW PROSPECTING");

        // Try finding existing ongoing crumb trails around
        // Note: not filtering by chunk indexes or anything for those reasons:
        // 1. Crumb Trail radius might be huge and possibly exceed a single chunk, so this could result in possible 25+ wasm calls for each possible chunk_index
        // 2. It is probably unlikely that more than ~100 crumb trails will be active in a single region at a time. At worst, it won't exceed the amount of CCU in this region.
        let trails_in_range = ctx
            .db
            .crumb_trail_state()
            .iter()
            .filter(|cr| cr.location().distance_to(player_location) <= cr.join_radius)
            .filter(|cr| {
                ctx //Make sure it's the same resource as the one we're prospecting
                    .db
                    .prospecting_state()
                    .crumb_trail_entity_id()
                    .filter(cr.entity_id)
                    .any(|s| s.prospecting_id == prospecting_id)
            });
        let mut best_trail: Option<CrumbTrailState> = None;
        let mut best_score = i32::MAX;
        for trail in trails_in_range {
            if trail.active_step < trail.crumb_radiuses.len() as i32 {
                // don't join trails with revealed resource as you won't be able to get contribution
                // this also prevents someone from completing a trail and rejoining it right away
                let score = trail.score(player_location);
                if score < best_score {
                    best_score = score;
                    best_trail = Some(trail);
                }
            }
        }

        // Found an existing trail to join
        if let Some(trail) = best_trail {
            player_prospecting = Some(ProspectingState {
                entity_id: actor_id,
                prospecting_id,
                crumb_trail_entity_id: trail.entity_id,
                completed_steps: 0,
                total_steps: (trail.crumb_locations.len() + 1) as i32,
                ongoing_step: trail.active_step,
                last_prospection_timestamp: ctx.timestamp,
                next_crumb_angle: Vec::new(), // this will be updated below
                contribution: 0,
                to_next_node: 0.0, // this will be updated below
            });
            log!("* Joining existing CrumbTrail {{0}}|~{}", trail.entity_id);
        } else {
            if let Some(mut new_trail) = CrumbTrailState::create(ctx, player_location, prospecting_id) {
                let mut contribution = 0;
                if new_trail.crumb_locations.is_empty() {
                    contribution = contribution_for_uncovered_breadcrumb(&prospecting_desc, contribution);
                }
                player_prospecting = Some(ProspectingState {
                    entity_id: actor_id,
                    prospecting_id,
                    crumb_trail_entity_id: new_trail.entity_id,
                    completed_steps: 0,
                    ongoing_step: 0,
                    total_steps: (new_trail.crumb_locations.len() + 1) as i32,
                    next_crumb_angle: Vec::new(), // this will be updated below
                    last_prospection_timestamp: ctx.timestamp,
                    contribution,
                    to_next_node: 0.0, // this will be updated below
                });

                ctx.db.crumb_trail_exposed_state().insert(CrumbTrailExposedState {
                    crumb_trail_entity_id: new_trail.entity_id,
                    exposed_locations: Vec::new(),
                    exposed_herd_entity_id: 0,
                });

                if new_trail.crumb_locations.is_empty() {
                    reveal_reward_for_zero_crumb_trail(ctx, &mut new_trail, &prospecting_desc);
                }

                ctx.db.crumb_trail_state().insert(new_trail);
            } else {
                // Did not find anything. Not an error, but nothing gets created either.
                return Ok(());
            }
        }
        log!("*");
    }

    let mut player_prospecting = player_prospecting.unwrap();
    let step = player_prospecting.ongoing_step as usize;
    // Check if the player is within the target area
    let mut crumb_trail = ctx
        .db
        .crumb_trail_state()
        .entity_id()
        .find(player_prospecting.crumb_trail_entity_id)
        .unwrap();
    let target_location = if step < crumb_trail.crumb_locations.len() {
        SmallHexTileMessage::from(crumb_trail.crumb_locations[step])
    } else {
        ctx.db
            .location_state()
            .entity_id()
            .find(crumb_trail.prize_entity_ids.first().unwrap())
            .unwrap()
            .coordinates()
    };

    // Step 0 is flagged at [Original Location] but checking for CrumbLocation[0]
    // If we have 4 steps:
    // Step 0 -> Looking for CrumbLocation[0], didn't reach any crumb
    // Step 1 -> Looking for CrumbLocation[1], reached Crumb #0
    // Step 2 -> Looking for CrumbLocation[2], reached Crumb #1
    // Step 3 -> Looking for CrumbLocation[3], reached Crumb #2
    // Step 4 -> Looking for PrizeLocation, reached Crumb #3
    let num_crumbs = crumb_trail.crumb_radiuses.len();
    let is_heading_torwards_reward = step >= num_crumbs;

    log!("* PROSPECTING RESULT");
    log!("* Step # {{0}} / {{1}}|~{}|~{}", step, num_crumbs);
    if is_heading_torwards_reward {
        log!("* Target Location: {{0}}|~{:?}", target_location,);
    } else {
        log!(
            "* Target Location: {{0}} Radius: {{1}}|~{:?}|~{}",
            target_location,
            crumb_trail.crumb_radiuses[step]
        );
    }
    log!("* Player Location: {{0}}|~{:?}", player_location);
    log!("* Distance: {{0}}|~{:?}", target_location.distance_to(player_location));
    if !is_heading_torwards_reward {
        log!(
            "* Success => {{0}}|~{}",
            target_location.distance_to(player_location) < crumb_trail.crumb_radiuses[step]
        );
    }
    log!("*");
    log!("**********************************************");

    let mut updated_trail = false;
    if !is_heading_torwards_reward {
        if target_location.distance_to(player_location) < crumb_trail.crumb_radiuses[step] {
            if step as i32 == crumb_trail.active_step {
                crumb_trail.active_step += 1;

                let mut exposed = ctx
                    .db
                    .crumb_trail_exposed_state()
                    .crumb_trail_entity_id()
                    .find(crumb_trail.entity_id)
                    .unwrap();

                // Expose the node we just visited
                exposed
                    .exposed_locations
                    .push(crumb_trail.crumb_locations[(crumb_trail.active_step - 1) as usize]);

                if crumb_trail.active_step as usize == num_crumbs {
                    // Also expose the final resource position, so it shows as auto-refreshing prospecting
                    exposed.exposed_locations.push(crumb_trail.prize_location);
                    if prospecting_desc.enemy_ai_desc_id != 0 {
                        // Spawn Herd
                        log!("* Spawning Prize Herd");
                        exposed.exposed_herd_entity_id = crumb_trail.spawn_herd_prize(ctx, &prospecting_desc);
                    } else {
                        // Spawn final clump
                        log!("* Replacing placeholder prize resource by official version");
                        crumb_trail.replace_prize_resources(ctx, &prospecting_desc);
                    }
                }
                ctx.db.crumb_trail_exposed_state().crumb_trail_entity_id().update(exposed);
            }
            player_prospecting.completed_steps += 1;
            player_prospecting.contribution =
                contribution_for_uncovered_breadcrumb(&prospecting_desc, player_prospecting.contribution);
            player_prospecting.ongoing_step += 1; //crumb_trail.active_step;           // for now, fast-track to the end but don't skip any step
            updated_trail = true;

            // Gain experience from that node
            let quantity = prospecting_desc.experience_per_node.quantity;
            if quantity > 0.0 {
                ExperienceState::add_experience_f32(ctx, actor_id, prospecting_desc.experience_per_node.skill_id, quantity);
            }

            if let Some(step_item_stacks) = &prospecting_desc.step_item_stacks {
                InventoryState::deposit_to_player_inventory_and_nearby_deployables(
                    ctx,
                    actor_id,
                    step_item_stacks,
                    |x| x.distance_to(player_location),
                    true,
                    || vec![player_location],
                    false,
                )?;
                post_reducer_update_cargo(ctx, actor_id);
            }
        }
    }
    player_prospecting.last_prospection_timestamp = ctx.timestamp;
    player_prospecting.next_crumb_angle = crumb_trail.angles_to_destination(player_location, player_prospecting.ongoing_step);
    player_prospecting.to_next_node = crumb_trail.distance_to_destination(location.coordinates_float(), player_prospecting.ongoing_step);
    ctx.db.prospecting_state().entity_id().insert_or_update(player_prospecting);

    if updated_trail {
        ctx.db.crumb_trail_state().entity_id().update(crumb_trail);
    }

    Ok(())
}
