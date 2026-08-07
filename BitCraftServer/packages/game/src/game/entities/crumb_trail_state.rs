use std::{collections::HashMap, f32::consts::PI};

use glam::Vec2;
use spacetimedb::{log, rand::Rng, ReducerContext, Table};

use crate::{
    agents::resources_regen::*, game::{
        coordinates::{FloatHexTile, SmallHexTile}, dimensions, game_state::{self, game_state_filters}, terrain_chunk::TerrainChunkCache, unity_helpers::vector2::Vector2,
    }, messages::{
        components::{
            CrumbTrailContributionLockState, CrumbTrailState, HerdState, ResourceState, crumb_trail_contribution_lock_state, herd_state,
        }, generic::resource_count, static_data::{ProspectingDesc, ResourceDesc, prospecting_desc, resource_clump_desc, resource_desc}, util::{OffsetCoordinatesSmallMessage, SmallHexTileMessage}, world_gen::WorldGenVector2,
    }, utils::iter_utils::GroupByAndCount,
};

impl CrumbTrailState {
    const EXTRA_DEBUG: bool = false;

    pub fn score(&self, actor_location: SmallHexTileMessage) -> i32 {
        actor_location.distance_to(self.location())
    }

    pub fn location(&self) -> SmallHexTileMessage {
        let location = SmallHexTileMessage::from(if self.active_step == 0 {
            self.original_location
        } else {
            self.crumb_locations[(self.active_step - 1) as usize]
        });
        location
    }

    pub fn create(ctx: &ReducerContext, location: SmallHexTileMessage, prospecting_id: i32) -> Option<Self> {
        struct Node {
            pub coord: SmallHexTileMessage,
            pub attempts: i32,
        }

        const TOTAL_MAX_ATTEMPTS: i32 = 250;
        const MAX_ATTEMPTS: i32 = 10;
        const FINAL_PRIZE_MAX_ATTEMPTS: i32 = 30;

        let entity_id = game_state::create_entity(ctx);
        let prospecting_desc = ctx.db.prospecting_desc().id().find(prospecting_id).unwrap();

        let target_crumb_count = ctx
            .rng()
            .gen_range(prospecting_desc.bread_crumb_count[0]..=prospecting_desc.bread_crumb_count[1]);

        log::info!("**********************************************");
        log::info!("*");
        log::info!("*");
        log::info!("* CREATING {target_crumb_count}-BREADCRUMBS TRAIL");
        log::info!("*");
        log::info!("*");
        log::info!("**********************************************");

        let mut terrain_cache = TerrainChunkCache::empty();

        // Placeholder prize resource placement info (ugh)
        let mut resource_desc: HashMap<i32, ResourceDesc> = HashMap::new();

        let clump_desc_extended;
        let mut occupied_tiles_hashes;
        if prospecting_desc.placeholder_resource_clump_id != 0 {
            resource_desc = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();
            occupied_tiles_hashes = PrecomputeOccupiedTiles::new(ctx);
            let clump_desc = ctx
                .db
                .resource_clump_desc()
                .id()
                .find(prospecting_desc.placeholder_resource_clump_id)
                .unwrap();
            clump_desc_extended = ResourceClumpDescExtended::new(clump_desc, &resource_desc);
        } else {
            occupied_tiles_hashes = PrecomputeOccupiedTiles::place_holder();
            let clump_desc = ctx.db.resource_clump_desc().iter().next().unwrap();
            clump_desc_extended = ResourceClumpDescExtended::place_holder(clump_desc);
        }

        let mut trail: Vec<Node> = Vec::new();
        trail.push(Node {
            coord: location,
            attempts: 0,
        });

        let mut total_attempts = 0;
        let deadzone = prospecting_desc.deadzone_angle_between_crumbs / 360.0 * PI;

        while trail.len() > 0 {
            total_attempts += 1;

            if total_attempts > TOTAL_MAX_ATTEMPTS {
                log::info!(
                    "CrumbTrailState::create aborted after {} total attempts for prospecting_id {}",
                    total_attempts,
                    prospecting_id
                );
                return None;
            }

            let current_trail_length = trail.len();
            let is_prize_node = current_trail_length > target_crumb_count as usize; // Note: Trail 0 is the prospecting point and not technically part of the crumbs

            {
                // Increase current attempts and get rid of the mut borrow
                let current_node = trail.get_mut((current_trail_length - 1) as usize).unwrap();
                current_node.attempts += 1;

                if current_node.attempts > if is_prize_node { FINAL_PRIZE_MAX_ATTEMPTS } else { MAX_ATTEMPTS } {
                    Self::extra_debug_log(format!("# Removing Node {{0}}|~{}", trail.len() - 1,).as_str());
                    trail.remove(trail.len() - 1);
                    continue;
                }
            }

            let current_node = trail.last().unwrap();
            Self::extra_debug_log(
                format!(
                    "Checking Node {{0}} at {{1}} (tries: {{2}})|~{}|~{:?}|~{}",
                    trail.len() - 1,
                    current_node.coord,
                    current_node.attempts
                )
                .as_str(),
            );

            let current_pos: Vector2 = current_node.coord.to_center_position_xz();
            let angle_radians = ctx.rng().gen_range(0.0..2.0 * PI);
            let length = ctx
                .rng()
                .gen_range(prospecting_desc.distance_between_bread_crumbs[0]..=prospecting_desc.distance_between_bread_crumbs[1])
                as f32
                * 3.333; // 10.0 is a large tile outer radius
            let delta = Vec2::from_angle(angle_radians) * length;
            let vec = Vec2 {
                x: current_pos.x,
                y: current_pos.y,
            } + delta;
            let next_pos = Vector2 { x: vec.x, y: vec.y };

            Self::extra_debug_log(format!("{{0}} -> {{1}}|~{:?}|~{:?}", current_pos, next_pos).as_str());
            // Check if angle is not within the dead angle
            if current_trail_length > 1 {
                let v1 = delta; // current crumb to next crumb
                let prev_v2 = trail[(current_trail_length - 2) as usize].coord.to_center_position_xz();
                let current_v2 = current_node.coord.to_center_position_xz();
                let v2 = Vec2 {
                    // current crumb to previous crumb
                    x: prev_v2.x - current_v2.x,
                    y: prev_v2.y - current_v2.y,
                };
                let angle = Vec2::angle_to(v1, v2);
                Self::extra_debug_log(
                    format!(
                        "v1 = {{0}}, v2 = {{1}}, angle = {{2}} deadzone = {{3}} dot = {{4}}|~{:?}|~{:?}|~{angle}|~{deadzone}|~{}",
                        v1,
                        v2,
                        v1.normalize().dot(v2.normalize())
                    )
                    .as_str(),
                );
                if angle.abs() < deadzone {
                    // the new point is within the dead angle, this is a failed attempt.
                    Self::extra_debug_log("denied, angle within the deadzone");
                    continue;
                }
            }

            let coord = SmallHexTileMessage::from_position(next_pos, dimensions::OVERWORLD);
            Self::extra_debug_log(format!("new node at {{0}}|~{:?}", coord).as_str());
            // Check if the new location water/ground state and biome are adequate
            if let Some(terrain_cell) = terrain_cache.get_terrain_cell(ctx, &coord.parent_large_tile()) {
                Self::extra_debug_log(format!("terrain cell biome: {{0}}|~{}", terrain_cell.biome()).as_str());
                let is_submerged = game_state_filters::is_submerged(ctx, &mut terrain_cache, coord);
                if is_submerged {
                    if !is_prize_node && !prospecting_desc.allow_aquatic_bread_crumb {
                        // denied, crumb is over water and that's not allowed
                        Self::extra_debug_log("denied, on water");
                        continue;
                    }
                }
                if !prospecting_desc.biome_requirements.contains(&terrain_cell.biome()) {
                    // denied, biome is not allowed
                    Self::extra_debug_log("denied, wrong biome");
                    continue;
                }

                let mut prize_location = OffsetCoordinatesSmallMessage::from(coord);
                let mut prize_entity_ids = Vec::new();

                if is_prize_node {
                    if prospecting_desc.enemy_ai_desc_id != 0 {
                        // Any additional herd-only checks come here
                        if is_submerged {
                            // denied, prize is over water and we don't have aquatic herds yet
                            Self::extra_debug_log("denied, herd on water");
                            continue;
                        }
                    } else {
                        // resources
                        // Check if prize resource can fit using the placeholder field
                        let hex_coordinates = coord;

                        let mut result = Vec::new();
                        let mut resources_to_delete = Vec::new();
                        Self::extra_debug_log("# trying to spawn final resource #");
                        Self::extra_debug_log(format!("clump_desc_extended => {{0}}|~{:?}", clump_desc_extended).as_str());
                        Self::extra_debug_log(format!("hex_coordinates => {{0}}|~{:?}", hex_coordinates).as_str());
                        Self::extra_debug_log(format!("resource_desc => {{0}}|~{:?}", resource_desc).as_str());
                        if try_spawn_resource_no_clump_info(
                            ctx,
                            &mut terrain_cache,
                            !prospecting_desc.is_aquatic_resource,
                            &clump_desc_extended,
                            hex_coordinates,
                            &mut occupied_tiles_hashes,
                            &resource_desc,
                            &mut resources_to_delete,
                            &mut result,
                        ) {
                            // This is a success, we can generate the trail
                            // Note: the placeholder resource will already be inserted at the end
                            Self::extra_debug_log(format!("RESULT => {{0}}|~{:?}", result).as_str());
                            Self::extra_debug_log(format!("resources_to_delete => {{0}}|~{:?}", resources_to_delete).as_str());
                            prize_location = OffsetCoordinatesSmallMessage::from(result[0].1);

                            // this is pretty much a resource regen/respawn code. We'll need to do a common function at some point.
                            // resource_regen:225
                            if result.len() > 0 {
                                let resources_to_delete_count = resources_to_delete.iter().group_by_and_count(|r| r.1);
                                for (entity_id, resource_id) in resources_to_delete {
                                    ResourceState::despawn(ctx, entity_id, resource_id);
                                }

                                for (resource_id, delete_count) in resources_to_delete_count {
                                    let mut count = ctx
                                        .db
                                        .resource_count()
                                        .resource_id()
                                        .find(&resource_id)
                                        .unwrap_or_else(|| panic!("No ResourceCount for resource_id: {}", resource_id));
                                    count.num_in_world -= delete_count as i32;
                                    ctx.db.resource_count().resource_id().update(count);
                                }

                                for &(resource_id, coordinates, direction) in &result {
                                    let resource_desc = resource_desc.get(&resource_id).unwrap();
                                    // TODO: mirror `ResourceCount` counters into WASM memory,
                                    //       update the in-memory version for each inserted resource,
                                    //       then do one `ResourceCount::update_by_resource_id` for each type
                                    //       after all insertions,
                                    //       rather than having `resource_spawn::spawn` update the counter each time.
                                    let resource_entity_id = ResourceState::spawn(
                                        ctx,
                                        None,
                                        resource_id,
                                        coordinates,
                                        direction,
                                        resource_desc.max_health,
                                        false,
                                        false,
                                    )
                                    .ok()
                                    .unwrap();

                                    prize_entity_ids.push(resource_entity_id);
                                }
                            } else {
                                Self::extra_debug_log("Denied, unable to place prize resource");
                                continue;
                            }
                        } else {
                            Self::extra_debug_log("Denied, prize resource does not fit location");
                            continue;
                        }
                    }

                    // Remove the first node which is the prospecting point and not part of the trail
                    trail.remove(0);
                    let crumb_locations = trail.iter().map(|node| node.coord.to_offset_coordinates()).collect();
                    let crumb_radiuses = trail
                        .iter()
                        .map(|_| {
                            ctx.rng()
                                .gen_range(prospecting_desc.bread_crumb_radius[0]..=prospecting_desc.bread_crumb_radius[1])
                        })
                        .collect();

                    let trail = CrumbTrailState {
                        entity_id,
                        original_location: OffsetCoordinatesSmallMessage::from(location),
                        crumb_locations,
                        crumb_radiuses,
                        prize_location,
                        active_step: 0,
                        prize_entity_ids,
                        join_radius: prospecting_desc.join_radius,
                        clean_up_counter: 0,
                    };
                    Self::extra_debug_log("# success #");

                    return Some(trail);
                } else {
                    // Node accepted.
                    Self::extra_debug_log("# valid node #");
                    trail.push(Node { coord, attempts: 0 });
                }
            } else {
                Self::extra_debug_log("denied, not in a chunk");
            }
        }

        // This was a failure
        None
    }

    fn extra_debug_log(str: &str) {
        if Self::EXTRA_DEBUG {
            log::info!("{}", str)
        }
    }

    pub fn replace_prize_resources(&mut self, ctx: &ReducerContext, prospecting_desc: &ProspectingDesc) {
        // Despawn Placeholder resources
        for entity_id in &self.prize_entity_ids {
            log::info!("despawning {entity_id}");
            ResourceState::despawn(ctx, *entity_id, 0); // prizes are not subject to equilibrium
        }

        let resource_desc = ctx.db.resource_desc().iter().map(|r| (r.id, r)).collect();
        let mut occupied_tiles_hashes = PrecomputeOccupiedTiles::new(ctx);
        let clump_desc = ctx.db.resource_clump_desc().id().find(prospecting_desc.resource_clump_id).unwrap();
        let clump_desc_extended = ResourceClumpDescExtended::new(clump_desc, &resource_desc);
        let mut result = Vec::new();
        let mut resources_to_delete = Vec::new();
        let mut terrain_cache = TerrainChunkCache::empty();
        let hex_coordinates = SmallHexTileMessage::from(self.prize_location);

        if !try_spawn_resource_no_clump_info(
            ctx,
            &mut terrain_cache,
            !prospecting_desc.is_aquatic_resource,
            &clump_desc_extended,
            hex_coordinates,
            &mut occupied_tiles_hashes,
            &resource_desc,
            &mut resources_to_delete,
            &mut result,
        ) {
            panic!("Place holder resource doesn't have the same footprint as final resource");
        }

        // this is pretty much a resource regen/respawn code. We'll need to do a common function at some point.
        // resource_regen:225
        self.prize_entity_ids.clear();
        if result.len() > 0 {
            for &(resource_id, coordinates, direction) in &result {
                let resource_desc = resource_desc.get(&resource_id).unwrap();
                // TODO: mirror `ResourceCount` counters into WASM memory,
                //       update the in-memory version for each inserted resource,
                //       then do one `ResourceCount::update_by_resource_id` for each type
                //       after all insertions,
                //       rather than having `resource_spawn::spawn` update the counter each time.
                let resource_entity_id = ResourceState::spawn(
                    ctx,
                    None,
                    resource_id,
                    coordinates,
                    direction,
                    resource_desc.max_health,
                    false,
                    false,
                )
                .ok()
                .unwrap();

                self.prize_entity_ids.push(resource_entity_id);

                let contribution_lock = CrumbTrailContributionLockState {
                    entity_id: resource_entity_id,
                    crumb_trail_entity_id: self.entity_id,
                };
                ctx.db.crumb_trail_contribution_lock_state().insert(contribution_lock);
            }
        }
    }

    pub fn angles_to_destination(&self, player_location: SmallHexTile, step: i32) -> Vec<f32> {
        let to_prize = step as usize >= self.crumb_locations.len();

        let a = player_location.to_center_position_xz();
        Self::extra_debug_log(format!("player {{0}} => {{1}}|~{:?}|~{:?}", player_location, a).as_str());
        let mut b = Vec::new();

        // atan2
        if to_prize {
            let tile = SmallHexTileMessage::from(self.prize_location);
            b.push(tile.to_center_position_xz() - a);
        } else {
            let radius = self.crumb_radiuses[step as usize];
            let tile = SmallHexTileMessage::from(self.crumb_locations[step as usize]);
            let t = tile.to_center_position_xz();
            let v = t - a;
            let p = WorldGenVector2 { x: v.y, y: -v.x }.normalized() * (radius as f32) * 3.33333;
            Self::extra_debug_log(format!("target {{0}} => {{1}} delta v => {{2}}|~{:?}|~{:?}|~{:?}", tile, t, v).as_str());
            Self::extra_debug_log(
                format!(
                    "perpendicular => {{0}} normalized => {{1}}|~{:?}|~{:?}",
                    WorldGenVector2 { x: v.y, y: -v.x },
                    WorldGenVector2 { x: v.y, y: -v.x }.normalized()
                )
                .as_str(),
            );
            Self::extra_debug_log(format!("t1 => {{0}} ({{1}})|~{:?}|~{:?}", t + p, SmallHexTileMessage::from_position(t + p, 1)).as_str());
            Self::extra_debug_log(format!("t2 => {{0}} ({{1}})|~{:?}|~{:?}", t - p, SmallHexTileMessage::from_position(t - p, 1)).as_str());
            b.push(t + p - a);
            b.push(t - p - a);
        }

        let angles: Vec<f32> = b
            .iter()
            .map(|delta| {
                let angle = f32::atan2(delta.y, delta.x);
                Self::extra_debug_log(format!("{{0}} => atan2 = {{1}}|~{:?}|~{angle}", delta).as_str());
                angle
            })
            .collect();
        angles
    }

    pub fn distance_to_destination(&self, player_location: FloatHexTile, step: i32) -> f32 {
        let to_prize = step as usize >= self.crumb_locations.len();

        let tile = if to_prize {
            SmallHexTileMessage::from(self.prize_location)
        } else {
            SmallHexTileMessage::from(self.crumb_locations[step as usize])
        };
        let tile_location = FloatHexTile::from(tile);

        player_location.distance_to(tile_location)
    }

    pub fn spawn_herd_prize(&mut self, ctx: &ReducerContext, prospecting_desc: &ProspectingDesc) -> u64 {
        let enemy_ai_id = prospecting_desc.enemy_ai_desc_id;
        let mut herd = HerdState::new(ctx, enemy_ai_id);
        let herd_entity_id = herd.entity_id;
        herd.population_variance = vec![0.0, 0.0];
        herd.crumb_trail_entity_id = self.entity_id;
        self.prize_entity_ids = vec![herd.entity_id];
        game_state::insert_location(ctx, herd.entity_id, self.prize_location);
        ctx.db.herd_state().insert(herd);
        herd_entity_id
    }
}
