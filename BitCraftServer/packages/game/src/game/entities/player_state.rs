use std::collections::HashMap;

use spacetimedb::{log, ReducerContext, Table};

use crate::agents::traveler_task_agent;
use crate::game::coordinates::{ChunkCoordinates, FloatHexTile, OffsetCoordinatesFloat};
use crate::game::discovery::Discovery;
use crate::game::game_state::{self, unix_ms};
use crate::game::handlers::player;
use crate::game::{claim_helper, coordinates::*, dimensions};
use crate::messages::components::{
    ability_state, building_state, rez_sick_long_term_state, AbilityState, AbilityType, ActionCooldown, ActiveBuffState,
    CharacterStatsState, InteriorPlayerCountState, InventoryState, MobileEntityState, PlayerActionState, PlayerState, StaminaState,
    TravelerTaskState,
};
use crate::messages::game_util::{ActiveBuff, ExperienceStack, ItemStack, LevelRequirement};
use crate::messages::static_data::*;
use crate::{
    active_buff_state, character_stats_state, claim_state, deployable_state, dimension_description_state, equipment_state,
    experience_state, exploration_chunks_state, knowledge_secondary_state, location_cache, mobile_entity_state, mounting_state,
    player_username_state, toolbar_state, unwrap_or_err, unwrap_or_return, KnowledgeState,
};

pub const INCAPACITATED_MESSAGE: &str = "You are incapacitated.";
pub const ANY_SKILL_ID: i32 = 1;

impl PlayerState {
    pub fn get_skill_level(ctx: &ReducerContext, entity_id: u64, skill_id: i32) -> i32 {
        let mut skill_id = skill_id;

        let experience_state = ctx.db.experience_state().entity_id().find(&entity_id).unwrap();

        // ANY skill means highest level skill
        if skill_id == ANY_SKILL_ID {
            skill_id = experience_state
                .experience_stacks
                .iter()
                .max_by(|a, b| a.quantity.cmp(&b.quantity))
                .unwrap()
                .skill_id;
        }

        let stack = experience_state.experience_stacks.iter().find(|s| s.skill_id == skill_id);
        if stack.is_none() {
            return 0;
        }

        let stack = stack.unwrap();

        return ExperienceStack::level_for_experience(stack.quantity);
    }

    pub fn meets_level_requirement(ctx: &ReducerContext, entity_id: u64, level_requirement: &LevelRequirement) -> bool {
        Self::get_skill_level(ctx, entity_id, level_requirement.skill_id) >= level_requirement.level
    }

    pub fn move_player_and_explore(
        ctx: &ReducerContext,
        entity_id: u64,
        start_coordinates: &FloatHexTile,
        target_coordinates: &FloatHexTile,
        stamina_delta: f32,
        is_running: bool,
        timestamp: Option<u64>,
    ) -> Result<(), String> {
        let start_large = start_coordinates.parent_large_tile();
        let target_large = target_coordinates.parent_large_tile();
        // Technically Chunks are not the same as ExploredChunks but whatever
        let previous_chunk = ChunkCoordinates::from(start_large);
        let entered_chunk = ChunkCoordinates::from(target_large);

        let dimension_desc_start = ctx
            .db
            .dimension_description_state()
            .dimension_id()
            .find(&start_coordinates.dimension);

        let dimension_desc_target = if start_coordinates.dimension == target_coordinates.dimension {
            dimension_desc_start.as_ref().unwrap().clone()
        } else {
            ctx.db
                .dimension_description_state()
                .dimension_id()
                .find(&target_coordinates.dimension)
                .unwrap()
        };

        //DAB Note: temp hack to identify what's causing players to move out of bounds
        if let Some(dimension_desc_start) = &dimension_desc_start {
            if (previous_chunk.x < dimension_desc_start.dimension_position_large_x as i32)
                | (previous_chunk.z < dimension_desc_start.dimension_position_large_z as i32)
                | (previous_chunk.x
                    >= dimension_desc_start.dimension_position_large_x as i32 + dimension_desc_start.dimension_size_large_x as i32)
                | (previous_chunk.z
                    >= dimension_desc_start.dimension_position_large_z as i32 + dimension_desc_start.dimension_size_large_z as i32)
            {
                return Err(format!(
                    "Move origin outside of world bounds! Origin: ({{0}} {{1}})|~{}|~{}",
                    start_coordinates.x, start_coordinates.z
                ));
            }
        }
        if (entered_chunk.x < dimension_desc_target.dimension_position_large_x as i32)
            | (entered_chunk.z < dimension_desc_target.dimension_position_large_z as i32)
            | (entered_chunk.x
                >= dimension_desc_target.dimension_position_large_x as i32 + dimension_desc_target.dimension_size_large_x as i32)
            | (entered_chunk.z
                >= dimension_desc_target.dimension_position_large_z as i32 + dimension_desc_target.dimension_size_large_z as i32)
        {
            return Err(format!(
                "Move origin target of world bounds! Target: ({{0}} {{1}})|~{}|~{}",
                target_coordinates.x, target_coordinates.z
            ));
        }

        if start_coordinates.dimension != target_coordinates.dimension {
            InteriorPlayerCountState::dec(ctx, dimension_desc_start.unwrap_or_default().dimension_network_entity_id);
            InteriorPlayerCountState::inc(ctx, dimension_desc_target.dimension_network_entity_id);
        }

        return Self::move_player_and_explore_unsafe(
            ctx,
            entity_id,
            start_coordinates,
            target_coordinates,
            stamina_delta,
            is_running,
            timestamp,
        );
    }

    pub fn move_player_and_explore_unsafe(
        ctx: &ReducerContext,
        entity_id: u64,
        start_coordinates: &FloatHexTile,
        target_coordinates: &FloatHexTile,
        stamina_delta: f32,
        is_running: bool,
        timestamp: Option<u64>,
    ) -> Result<(), String> {
        let start_large = start_coordinates.parent_large_tile();
        let target_large = target_coordinates.parent_large_tile();
        // Technically Chunks are not the same as ExploredChunks but whatever
        let previous_chunk = ChunkCoordinates::from(start_large);
        let entered_chunk = ChunkCoordinates::from(target_large);

        // Don't explore non-overworld dimensions
        let in_overworld = target_coordinates.dimension == dimensions::OVERWORLD;
        if in_overworld & ((previous_chunk.x != entered_chunk.x) | (previous_chunk.z != entered_chunk.z)) {
            let mut exploration_chunks = unwrap_or_err!(
                ctx.db.exploration_chunks_state().entity_id().find(&entity_id),
                "Missing exploration_chunks_state in move_player_and_explore"
            );
            if exploration_chunks.explore_chunk(ctx, &entered_chunk, None) {
                PlayerState::discover_ruins_in_chunk(ctx, entity_id, entered_chunk);
                ctx.db.exploration_chunks_state().entity_id().update(exploration_chunks);
            }
        }

        // update location
        let start_offset_coordinates = OffsetCoordinatesFloat::from(start_coordinates);
        let target_offset_coordinates = OffsetCoordinatesFloat::from(target_coordinates);
        let mobile_entity = MobileEntityState {
            entity_id,
            // IMPORTANT: currently having negative or zero coordinates in here causes weird issues.
            // One known one is that we can't add negative numbers in our subscription queries.
            // Being at exactly 0,0 may cause some floating point conversion issue or something not sure.
            chunk_index: FloatHexTile::from(OffsetCoordinatesFloat {
                x: start_offset_coordinates.x.clamp(1, i32::MAX),
                z: start_offset_coordinates.z.clamp(1, i32::MAX),
                dimension: target_offset_coordinates.dimension,
            })
            .chunk_coordinates()
            .chunk_index(),
            timestamp: timestamp.unwrap_or_else(|| unix_ms(ctx.timestamp)),
            location_x: start_offset_coordinates.x.clamp(1, i32::MAX),
            location_z: start_offset_coordinates.z.clamp(1, i32::MAX),
            destination_x: target_offset_coordinates.x.clamp(1, i32::MAX),
            destination_z: target_offset_coordinates.z.clamp(1, i32::MAX),
            dimension: target_offset_coordinates.dimension,
            is_running,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        // Update both action state layers when changing chunk (whether in overworld or interiors)
        PlayerActionState::update_chunk_index_on_all_layers(ctx, entity_id, mobile_entity.chunk_index);

        // For now, walking into a claim cures long term rez sickness
        if ctx.db.rez_sick_long_term_state().entity_id().find(entity_id).is_some() {
            if let Some(claim_tile) = claim_helper::get_claim_on_tile(ctx, mobile_entity.coordinates()) {
                let claim = unwrap_or_err!(
                    ctx.db.claim_state().entity_id().find(claim_tile.claim_id),
                    "Missing claim_state in move_player_and_explore"
                );
                let heals_rez_sickness;
                if claim.owner_player_entity_id == 0 {
                    // Check if owner building is the ancient ruins one.
                    let building = unwrap_or_err!(
                        ctx.db.building_state().entity_id().find(claim.owner_building_entity_id),
                        "Missing building_state in move_player_and_explore"
                    );
                    let building_desc = unwrap_or_err!(
                        ctx.db.building_desc().id().find(building.building_description_id),
                        "Missing building_desc in move_player_and_explore"
                    );
                    heals_rez_sickness = building_desc.has_category(ctx, BuildingCategory::Bed);
                } else {
                    heals_rez_sickness = true;
                }
                if heals_rez_sickness {
                    let mut active_buff_state = unwrap_or_err!(
                        ctx.db.active_buff_state().entity_id().find(entity_id),
                        "Missing active_buff_state in move_player_and_explore"
                    );
                    if let Some(debuff) = active_buff_state.active_buff_of_category(ctx, BuffCategory::RezSicknessLongTerm) {
                        active_buff_state.remove_active_buff(ctx, debuff.buff_id);
                        ctx.db.active_buff_state().entity_id().update(active_buff_state);
                    }
                    // remove rez sickness entry
                    ctx.db.rez_sick_long_term_state().entity_id().delete(entity_id);
                }
            }
        }

        ctx.db.mobile_entity_state().entity_id().update(mobile_entity);

        // Discover claim under feet
        if let Some(claim_tile) = claim_helper::get_claim_on_tile(ctx, target_coordinates.into()) {
            let mut discovery = Discovery::new(entity_id);
            discovery.acquire_claim(ctx, claim_tile.claim_id);
            discovery.commit(ctx);
        }

        StaminaState::add_player_stamina(ctx, entity_id, stamina_delta);

        Ok(())
    }

    pub fn discover_ruins_in_chunk(ctx: &ReducerContext, entity_id: u64, chunk_coordinates: ChunkCoordinates) {
        let ruins = unwrap_or_return!(
            ctx.db.location_cache().version().find(&0),
            "Missing location_cache in discover_ruins_in_chunk"
        )
        .all_ruins;

        // learn about new ruins locations
        let ruins_coordinates: Vec<SmallHexTile> = ruins
            .iter()
            .filter(|r| ChunkCoordinates::from(r.coordinates) == chunk_coordinates)
            .map(|b| b.coordinates)
            .collect();

        let mut discovery = Discovery::new(entity_id);

        for coord in ruins_coordinates {
            discovery.acquire_ruins(ctx, OffsetCoordinatesSmall::from(coord));
        }

        discovery.commit(ctx);
    }

    pub fn collect_deployable_stats(ctx: &ReducerContext, deployable_desc_id: i32, bonuses: &mut HashMap<CharacterStatType, (f32, f32)>) {
        let deployable_desc = ctx.db.deployable_desc_v4().id().find(&deployable_desc_id).unwrap();
        for stat_delta in &deployable_desc.stats {
            let entry = bonuses.entry(stat_delta.id).or_insert((0.0, 0.0));
            if stat_delta.is_pct {
                *entry = (entry.0, entry.1 + stat_delta.value);
            } else {
                *entry = (entry.0 + stat_delta.value, entry.1);
            }
        }
    }

    fn collect_knowledge_stats(ctx: &ReducerContext, player_entity_id: u64, bonuses: &mut HashMap<CharacterStatType, (f32, f32)>) {
        let knowledge_bonuses: HashMap<i32, Vec<CsvStatEntry>> = ctx
            .db
            .knowledge_stat_modifier_desc()
            .iter()
            .map(|ksmd| (ksmd.secondary_knowledge_id, ksmd.stats))
            .collect();

        let acquired_knowledges: Vec<i32> = ctx
            .db
            .knowledge_secondary_state()
            .entity_id()
            .find(player_entity_id)
            .unwrap()
            .entries
            .iter()
            .filter_map(|k| if k.state == KnowledgeState::Acquired { Some(k.id) } else { None })
            .collect();

        for (knowledge_id, stat_list) in knowledge_bonuses {
            if acquired_knowledges.contains(&knowledge_id) {
                for stat_delta in stat_list {
                    let entry = bonuses.entry(stat_delta.id).or_insert((0.0, 0.0));
                    if stat_delta.is_pct {
                        *entry = (entry.0, entry.1 + stat_delta.value);
                    } else {
                        *entry = (entry.0 + stat_delta.value, entry.1);
                    }
                }
            }
        }
    }

    pub fn get_stat(ctx: &ReducerContext, entity_id: u64, stat: CharacterStatType) -> f32 {
        let stats = ctx.db.character_stats_state().entity_id().find(&entity_id).unwrap();
        stats.get(stat)
    }

    pub fn owns_claim(ctx: &ReducerContext, player_entity_id: u64) -> bool {
        ctx.db
            .claim_state()
            .owner_player_entity_id()
            .filter(player_entity_id)
            .next()
            .is_some()
    }

    pub fn get_hunting_weapon_type() -> i32 {
        // HARD_CODED: weapontype 7 is for hunting [skill 9], therefore it's slot 6. Ugh.
        7
    }

    pub fn get_combat_weapon_type(ctx: &ReducerContext) -> i32 {
        ctx.db.parameters_desc_v2().version().find(0).unwrap().default_num_toolbelt_pockets
    }

    pub fn get_hunting_weapon(ctx: &ReducerContext, actor_id: u64) -> Option<ItemStack> {
        InventoryState::get_player_toolbelt(ctx, actor_id)
            .unwrap()
            .get_pocket_contents((Self::get_hunting_weapon_type() - 1) as usize)
    }

    pub fn get_combat_weapon(ctx: &ReducerContext, actor_id: u64) -> Option<ItemStack> {
        InventoryState::get_player_toolbelt(ctx, actor_id)
            .unwrap()
            .get_pocket_contents((Self::get_combat_weapon_type(ctx) - 1) as usize)
    }

    pub fn init_toolbelt(ctx: &ReducerContext, actor_id: u64, item_id: i32) {
        // Add all actions related to that item
        // (for now only weapons give actions; ideally we might want to list actions per item)
        if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&item_id) {
            if let Some(weapon_type) = ctx.db.weapon_type_desc().id().find(&weapon.weapon_type) {
                // All player potential actions, mapped on a toolbar or not
                // Reset Hunting or Combat toolbar (for now)
                let mut toolbar_abilities = Vec::new();
                PlayerState::collect_stats(ctx, actor_id);

                let mut i = 1;
                for combat_action in ctx.db.combat_action_desc_v3().iter() {
                    if combat_action.learned_by_player
                        && combat_action.weapon_type_requirements.contains(&weapon_type.id)
                        && !combat_action.auto_cast
                    {
                        // TEMP - assign hunting or combat abilities
                        let _ = player::ability_set::reduce(ctx, actor_id, 0, i, AbilityType::CombatAction(combat_action.id));
                        let created_ability = ctx
                            .db
                            .ability_state()
                            .owner_entity_id()
                            .filter(actor_id)
                            .find(|a| a.ability == AbilityType::CombatAction(combat_action.id))
                            .unwrap();
                        toolbar_abilities.push(created_ability.entity_id); // temporary until we stop supporting old UI
                        i += 1;

                        // Any ability related to the new weapon will have a maxed out cooldown on swapping weapon
                        let mut ability = ctx
                            .db
                            .ability_state()
                            .owner_entity_id()
                            .filter(actor_id)
                            .find(|a| a.ability == AbilityType::CombatAction(combat_action.id))
                            .unwrap();
                        let (cooldown_multiplier, weapon_cooldown_multiplier) =
                            CharacterStatsState::get_cooldown_and_weapon_cooldown_multipliers(ctx, actor_id, weapon_type.hunting);

                        ability.set_combat_action_cooldown(
                            &combat_action,
                            cooldown_multiplier,
                            weapon_cooldown_multiplier,
                            ctx.timestamp,
                            true,
                        );
                        ctx.db.ability_state().entity_id().update(ability);
                    }
                }
                // temporary until we stop supporting old UI
                let mut toolbar = ctx
                    .db
                    .toolbar_state()
                    .owner_entity_id()
                    .filter(actor_id)
                    .find(|t| t.index == 0)
                    .unwrap();
                toolbar.actions = toolbar_abilities;
                ctx.db.toolbar_state().entity_id().update(toolbar);
            }
        }
    }

    pub fn on_added_to_toolbelt(ctx: &ReducerContext, actor_id: u64, item_id: i32) {
        if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&item_id) {
            if let Some(weapon_type) = ctx.db.weapon_type_desc().id().find(&weapon.weapon_type) {
                PlayerState::collect_stats(ctx, actor_id);

                for combat_action in ctx.db.combat_action_desc_v3().iter() {
                    if combat_action.learned_by_player
                        && combat_action.weapon_type_requirements.contains(&weapon_type.id)
                        && combat_action.auto_cast
                    {
                        let _ = ctx.db.ability_state().try_insert(AbilityState {
                            entity_id: game_state::create_entity(ctx),
                            owner_entity_id: actor_id,
                            ability: AbilityType::CombatAction(combat_action.id),
                            cooldown: ActionCooldown {
                                timestamp: 0,
                                cooldown: 0.0,
                            },
                        });
                    }
                }

                // Reset all combat abilities when equipping a new weapon
                for mut ability in ctx.db.ability_state().owner_entity_id().filter(actor_id) {
                    match ability.ability {
                        AbilityType::CombatAction(id) => {
                            let combat_action = ctx.db.combat_action_desc_v3().id().find(id).unwrap();

                            // Default is combat weapon, unless it's a huntable enemy
                            let is_huntable = weapon_type.hunting;
                            let (cooldown_multiplier, weapon_cooldown_multiplier) =
                                CharacterStatsState::get_cooldown_and_weapon_cooldown_multipliers(ctx, actor_id, is_huntable);
                            ability.set_combat_action_cooldown(
                                &combat_action,
                                cooldown_multiplier,
                                weapon_cooldown_multiplier,
                                ctx.timestamp,
                                true,
                            );
                            ability.update(ctx);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn on_removed_from_toolbelt(ctx: &ReducerContext, actor_id: u64, item_id: i32) {
        // Remove all actions related to that item
        if let Some(weapon) = ctx.db.weapon_desc().item_id().find(&item_id) {
            if let Some(weapon_type) = ctx.db.weapon_type_desc().id().find(&weapon.weapon_type) {
                // Find active toolbar (hunting or combat) and clear abilities from it
                let toolbar_index = if weapon_type.hunting { 0 } else { 1 };
                let mut toolbar = ctx
                    .db
                    .toolbar_state()
                    .owner_entity_id()
                    .filter(&actor_id)
                    .find(|t| t.index == toolbar_index)
                    .unwrap();
                toolbar.actions.clear();
                ctx.db.toolbar_state().entity_id().update(toolbar);

                PlayerState::collect_stats(ctx, actor_id);

                // Clean-up auto attack ability for previously equipped weapon if not under cooldown
                AbilityState::clean_up_unmapped_expired_abilities(ctx, actor_id);
            }
        }
        // DAB Note: eventually toolbar will be handled manually by the player
    }

    pub fn on_updated_toolbelt(ctx: &ReducerContext, actor_id: u64, previous_item_id: i32, new_item_id: i32) {
        let prev_weapon = ctx.db.weapon_desc().item_id().find(&previous_item_id);
        let new_weapon = ctx.db.weapon_desc().item_id().find(&new_item_id);
        if prev_weapon.is_some() && new_weapon.is_some() {
            //In order to prevent resetting cooldowns when swapping weapons, we do this for same weapon types now
            Self::on_removed_from_toolbelt(ctx, actor_id, previous_item_id);
            Self::on_added_to_toolbelt(ctx, actor_id, new_item_id);
        } else if !(prev_weapon.is_none() && new_weapon.is_none()) {
            log::error!("A weapon item and a non-weapon item were swapped on a toolbelt slot, which is UNACCEPTABLE!");
        }
    }

    pub fn collect_stats_with_uncommited_buffs(ctx: &ReducerContext, active_buffs: &ActiveBuffState) {
        let player_entity_id = active_buffs.entity_id;
        let equipment = ctx.db.equipment_state().entity_id().find(&player_entity_id).unwrap();

        let mut bonuses = HashMap::new();
        // Collect stats from equipment
        equipment.collect_stats(ctx, &mut bonuses);
        // Collect stats from buffs
        active_buffs.collect_buff_stats(ctx, &mut bonuses);
        /*
        // Collect stats from paving player is on [DISABLED FOR NOW]
        //let coords = ctx.db.mobile_entity_state().entity_id().find(&player_entity_id).unwrap().coordinates();
        PavedTileState::collect_stats(&coords, &mut bonuses);
        */
        // Collect stats from deployable
        if let Some(mounting) = ctx.db.mounting_state().entity_id().find(&player_entity_id) {
            if let Some(deployable) = ctx.db.deployable_state().entity_id().find(&mounting.deployable_entity_id) {
                Self::collect_deployable_stats(ctx, deployable.deployable_description_id, &mut bonuses);
            }
        }
        // Collect stats from knowledges
        Self::collect_knowledge_stats(ctx, player_entity_id, &mut bonuses);

        // Eventually, collect stats from trinkets or possibly collectibles
        // ...

        // Apply stats to base Stats (and clamp within min/max)
        let mut new_stats = CharacterStatsState::new(ctx, player_entity_id);
        for (index, delta) in bonuses {
            let stat_desc = ctx.db.character_stat_desc().stat_type().find(index as i32).unwrap();
            let cumulated_value = new_stats.get(index) + delta.0;
            let cumulated_pct = 1.0 + delta.1;
            let final_value = f32::clamp(cumulated_value * cumulated_pct, stat_desc.min_value, stat_desc.max_value);
            new_stats.set(index, final_value);
        }

        // Queue update with all different stats
        let stats = ctx.db.character_stats_state().entity_id().find(&player_entity_id).unwrap();
        if !new_stats.equals(&stats) {
            ctx.db.character_stats_state().entity_id().update(new_stats);
        }

        // Possibly remove the rez sickness from the table
        if ctx.db.rez_sick_long_term_state().entity_id().find(player_entity_id).is_some() {
            if !ActiveBuff::has_active_buff_of_category(ctx, player_entity_id, BuffCategory::RezSicknessLongTerm) {
                ctx.db.rez_sick_long_term_state().entity_id().delete(player_entity_id);
            }
        }
    }

    pub fn collect_stats(ctx: &ReducerContext, player_entity_id: u64) {
        if let Some(active_buff_state) = ctx.db.active_buff_state().entity_id().find(&player_entity_id) {
            Self::collect_stats_with_uncommited_buffs(ctx, &active_buff_state);
        }
    }

    pub fn username(&self, ctx: &ReducerContext) -> String {
        return ctx.db.player_username_state().entity_id().find(&self.entity_id).unwrap().username;
    }

    pub fn username_by_id(ctx: &ReducerContext, player_entity_id: u64) -> Option<String> {
        return ctx
            .db
            .player_username_state()
            .entity_id()
            .find(&player_entity_id)
            .map(|u| u.username);
    }

    pub fn refresh_traveler_tasks(&mut self, ctx: &ReducerContext) {
        let next_task_refresh = traveler_task_agent::next_tick(ctx);
        if self.traveler_tasks_expiration >= next_task_refresh {
            // The current tasks are still active; nothing to refresh
            return;
        }
        self.traveler_tasks_expiration = next_task_refresh;

        TravelerTaskState::delete_all_for_player(ctx, self.entity_id);
        // Need to create new tasks for the player
        let requests = TravelerTaskState::generate_npc_requests_hashmap(ctx);
        let tasks_per_npc = ctx.db.parameters_desc_v2().version().find(0).unwrap().traveler_tasks_per_npc;
        TravelerTaskState::generate_all_for_player(ctx, self.entity_id, &requests, tasks_per_npc, next_task_refresh);
    }
}
