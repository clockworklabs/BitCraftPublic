use crate::game::coordinates::OffsetCoordinatesSmall;
use crate::game::discovery::autogen::_discovery::generate_knowledges;
use crate::game::discovery::Discovery;
use crate::game::game_state;
use crate::game::handlers::player;
use crate::inter_module::send_inter_module_message;
use crate::location_cache;
use crate::messages::game_util::{ItemStack, ItemType};
use crate::messages::generic::world_region_state;
use crate::messages::inter_module::{OnRegionPlayerCreatedMsg, PlayerCreateMsg};
use crate::messages::static_data::*;
use crate::messages::util::SmallHexTileMessage;
use crate::{
    game::{
        coordinates::{ChunkCoordinates, OffsetCoordinatesFloat},
        dimensions,
        game_state::insert_location_float,
    },
    messages::{
        components::*,
        game_util::{ActiveBuff, ExperienceStack, OnlineTimestamp},
        static_data::{CollectibleDesc, EquipmentSlot},
    },
    unwrap_or_err,
};
use spacetimedb::rand::seq::SliceRandom;
use spacetimedb::{log, ReducerContext, Table};
use std::collections::HashSet;

use crate::{
    game::{game_state::create_entity, location_cache::LocationCache},
    messages::components::UserState,
};

pub fn process_message_on_destination(ctx: &ReducerContext, player_create_request: PlayerCreateMsg) -> Result<(), String> {
    if ctx.db.user_state().identity().find(player_create_request.identity).is_some() {
        log::info!("Player already exists.");
        return Ok(());
    }

    let player_entity_id = create_entity(ctx);
    UserState::insert_shared(
        ctx,
        UserState {
            entity_id: player_entity_id,
            identity: player_create_request.identity,
            can_sign_in: false,
        },
        super::InterModuleDestination::Global,
    );

    create_player(ctx, player_entity_id)?;

    Ok(())
}

fn find_spawn_position(ctx: &ReducerContext, location_cache: &LocationCache) -> Option<SmallHexTileMessage> {
    let valid_spawn = &location_cache.spawn_locations;
    if valid_spawn.len() == 0 {
        log::error!("No spawn points found. Is the world empty?");
        return None;
    }

    let mut spawn_coordinates = *valid_spawn.choose(&mut ctx.rng()).unwrap();
    let mut walkable_coordinates = None;
    let mut perimeter_coordinates = None;
    spawn_coordinates.dimension = dimensions::OVERWORLD;
    for _ in 0..100 {
        let mut footprint_type = -1;
        for footprint in FootprintTileState::get_at_location(ctx, &spawn_coordinates) {
            if footprint.footprint_type == FootprintType::Hitbox {
                footprint_type = FootprintType::Hitbox as i32;
                break;
            }
            if footprint.footprint_type == FootprintType::Perimeter {
                footprint_type = FootprintType::Perimeter as i32;
                continue;
            }
            if footprint.footprint_type == FootprintType::Walkable && footprint_type == -1 {
                footprint_type = FootprintType::Walkable as i32;
                continue;
            }
        }
        if footprint_type == -1 {
            // no footprint, or irrelevant footprints, therefore we keep the selected spawn coordinate
            return Some(spawn_coordinates);
        }

        if perimeter_coordinates.is_none() {
            if footprint_type == FootprintType::Perimeter as i32 {
                // backup spawn coordinate
                perimeter_coordinates = Some(spawn_coordinates);
            }
            if footprint_type == FootprintType::Walkable as i32 && walkable_coordinates.is_none() {
                // backup spawn coordinate
                walkable_coordinates = Some(spawn_coordinates);
            }
        }
        // try again
        spawn_coordinates = *valid_spawn.choose(&mut ctx.rng()).unwrap();
    }

    if perimeter_coordinates.is_some() {
        return perimeter_coordinates;
    }

    if walkable_coordinates.is_some() {
        return walkable_coordinates;
    }

    Some(spawn_coordinates)
}

fn create_player(ctx: &ReducerContext, entity_id: u64) -> Result<u64, String> {
    let username = format!("player{}", entity_id);
    let username_lowercase = username.to_lowercase();
    let default_pocket_volume = 6000;

    let location_cache = unwrap_or_err!(
        ctx.db.location_cache().version().find(&0),
        "Location Cache not built. Did you generate a world?"
    );

    let spawn_coordinates = unwrap_or_err!(find_spawn_position(ctx, &location_cache), "Unable to find a spawn location");

    let spawn_offset = OffsetCoordinatesSmall::from(spawn_coordinates);
    let spawn_offset_float = OffsetCoordinatesFloat::from(spawn_offset);
    insert_location_float(ctx, entity_id, spawn_offset_float);

    ctx.db
        .move_validation_strike_counter_state()
        .try_insert(MoveValidationStrikeCounterState {
            entity_id,
            validation_failure_timestamps: Vec::with_capacity(0),
        })?;

    let player_state = PlayerState {
        entity_id,
        signed_in: false,
        teleport_location: TeleportLocation {
            location: spawn_offset,
            location_type: TeleportLocationType::BirthLocation,
        },
        time_played: 0,
        session_start_timestamp: 0,
        time_signed_in: 0,
        sign_in_timestamp: 0,
        traveler_tasks_expiration: 0,
    };

    ctx.db.player_prefs_state().try_insert(PlayerPrefsState {
        entity_id,
        default_deployable_collectible_id: 0,
    })?;

    ctx.db.experience_state().try_insert(ExperienceState {
        entity_id,

        experience_stacks: ctx
            .db
            .skill_desc()
            .iter()
            .map(|skill| ExperienceStack {
                skill_id: skill.id,
                quantity: 0,
            })
            .collect(),
    })?;

    let chunk_index = TerrainChunkState::chunk_index_from_coords(&ChunkCoordinates::from(spawn_coordinates));

    ctx.db.player_action_state().try_insert(PlayerActionState {
        auto_id: 0,
        entity_id: entity_id,
        action_type: PlayerActionType::None,
        layer: PlayerActionLayer::Base,
        target: None,
        recipe_id: None,
        start_time: game_state::unix_ms(ctx.timestamp),
        duration: 0,
        last_action_result: PlayerActionResult::Success,
        client_cancel: false,
        chunk_index,
        _pad: 0,
    })?;

    ctx.db.player_action_state().try_insert(PlayerActionState {
        auto_id: 0,
        entity_id: entity_id,
        action_type: PlayerActionType::None,
        layer: PlayerActionLayer::UpperBody,
        target: None,
        recipe_id: None,
        start_time: game_state::unix_ms(ctx.timestamp),
        duration: 0,
        last_action_result: PlayerActionResult::Success,
        client_cancel: false,
        chunk_index,
        _pad: 0,
    })?;

    // activate first collectible of each type
    let mut collectible_types = HashSet::new();

    // grant initial collectible loadout
    let mut granted_collectibles: Vec<CollectibleDesc> = ctx.db.collectible_desc().iter().filter(|c| c.starting_loadout).collect();

    granted_collectibles.sort_by_key(|col| col.id);

    let mut vault = VaultState {
        entity_id,
        collectibles: Vec::new(),
    };

    for collectible in granted_collectibles {
        let activated = collectible.max_equip_count > 0
            && collectible.collectible_type != CollectibleType::ClothesHead
            && collectible.collectible_type != CollectibleType::ClothesTorso
            && collectible.collectible_type != CollectibleType::ClothesBelt
            && collectible.collectible_type != CollectibleType::ClothesLegs
            && collectible.collectible_type != CollectibleType::ClothesArms
            && collectible.collectible_type != CollectibleType::ClothesFeet
            && collectible.collectible_type != CollectibleType::ClothesCape
            && collectible_types.insert(collectible.collectible_type);

        vault.collectibles.push(VaultCollectible {
            id: collectible.id,
            count: 1,
            activated,
        });
    }

    ctx.db.vault_state().try_insert(vault)?;

    let region = ctx.db.world_region_state().id().find(&0).unwrap();
    let world_width = region.world_width_chunks();
    let mut exploration_state = ExplorationChunksState::new(ctx, entity_id, Some(region));
    let explored_chunks_coordinates = ChunkCoordinates::from(spawn_coordinates).surrounding_and_including(ctx);
    for chunk in &explored_chunks_coordinates {
        exploration_state.explore_chunk(ctx, chunk, Some(world_width));
    }
    ctx.db.exploration_chunks_state().try_insert(exploration_state)?;

    generate_knowledges(ctx, entity_id);

    // discover all ruins in starting chunks
    for chunk in explored_chunks_coordinates {
        let chunk_coordinates = ChunkCoordinates {
            x: chunk.x,
            z: chunk.z,
            dimension: chunk.dimension,
        };
        PlayerState::discover_ruins_in_chunk(ctx, entity_id, chunk_coordinates);
    }

    // Grant "default" knowledge and starting equipment to player
    let secondary_id = 100000;

    let weapon_equipment_id = 2076275038; // Training Sword
    let torso_equipment_id = 1150019; // Grass Shirt
    let leg_equipment_id = 1183376713; // Grass Waistwrap
    let feet_equipment_id = 1150061; // Grass Sandals
    let hex_coin_id = 1; // Hex Coin

    let mut discovery = Discovery::new(entity_id);
    discovery.acquire_secondary(ctx, secondary_id);
    discovery.acquire_item(ctx, weapon_equipment_id);
    discovery.acquire_item(ctx, torso_equipment_id);
    discovery.acquire_item(ctx, leg_equipment_id);
    discovery.acquire_item(ctx, feet_equipment_id);
    discovery.acquire_item(ctx, hex_coin_id);
    discovery.commit(ctx);

    let equipment = EquipmentState {
        entity_id,
        equipment_slots: vec![
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::MainHand, // Obsolete for now
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::OffHand, // Obsolete for now
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::HeadArtifact,
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::TorsoArtifact,
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::HandArtifact, // Obsolete for now
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::FeetArtifact, // Obsolete for now
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::HeadClothing,
            },
            EquipmentSlot {
                item: Some(ItemStack::new(ctx, torso_equipment_id, ItemType::Item, 1)),
                primary: EquipmentSlotType::TorsoClothing,
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::HandClothing,
            },
            EquipmentSlot {
                item: None,
                primary: EquipmentSlotType::BeltClothing,
            },
            EquipmentSlot {
                item: Some(ItemStack::new(ctx, leg_equipment_id, ItemType::Item, 1)),
                primary: EquipmentSlotType::LegClothing,
            },
            EquipmentSlot {
                item: Some(ItemStack::new(ctx, feet_equipment_id, ItemType::Item, 1)),
                primary: EquipmentSlotType::FeetClothing,
            },
        ],
    };

    //toolbelt
    let params = ctx.db.parameters_desc_v2().version().find(&0).unwrap();
    let num_toolbelt_pockets = params.default_num_toolbelt_pockets;
    if !InventoryState::new_with_index(
        ctx,
        num_toolbelt_pockets,
        default_pocket_volume,
        0, // no cargo on toolbelt
        1, // Toolbelt is index 1 for player
        num_toolbelt_pockets,
        entity_id,
        0,
        None,
    ) {
        return Err("Failed to create player toolbelt inventory".into());
    }

    // Force equip mallet & cooking pot, which are no longer official items
    let mut toolbelt_inventory = InventoryState::get_player_toolbelt(ctx, entity_id).unwrap();
    let mallet = ctx.db.tool_desc().iter().find(|t| t.tool_type == 14 && t.level == 1).unwrap();
    let cooking_pot = ctx.db.tool_desc().iter().find(|t| t.tool_type == 11 && t.level == 1).unwrap();
    toolbelt_inventory.set_at(
        mallet.tool_type as usize - 1,
        Some(ItemStack::new(ctx, mallet.item_id, ItemType::Item, 1)),
    );
    toolbelt_inventory.set_at(
        cooking_pot.tool_type as usize - 1,
        Some(ItemStack::new(ctx, cooking_pot.item_id, ItemType::Item, 1)),
    );
    ctx.db.inventory_state().entity_id().update(toolbelt_inventory);

    //wallet
    let num_wallet_pockets = 1;
    if !InventoryState::new_with_index(
        ctx,
        num_wallet_pockets,
        default_pocket_volume,
        0, // no cargo in wallet
        2, // Wallet is index 2 for player
        num_wallet_pockets,
        entity_id,
        0,
        None,
    ) {
        return Err("Failed to create player wallet inventory".into());
    }

    let mut active_buffs: Vec<ActiveBuff> = ctx
        .db
        .buff_desc()
        .iter()
        .map(|buff| ActiveBuff {
            buff_id: buff.id,
            buff_start_timestamp: OnlineTimestamp { value: 0 },
            buff_duration: 0,
            values: Vec::new(),
        })
        .collect();
    active_buffs.sort_by(|a, b| a.buff_id.cmp(&b.buff_id));

    ctx.db.equipment_state().try_insert(equipment)?;

    let active_buff_state = ActiveBuffState { entity_id, active_buffs };
    ctx.db.active_buff_state().try_insert(active_buff_state)?;
    ctx.db.player_state().try_insert(player_state)?;
    ctx.db
        .character_stats_state()
        .try_insert(CharacterStatsState::new(ctx, entity_id))?;
    ctx.db
        .player_username_state()
        .try_insert(PlayerUsernameState { entity_id, username })?;
    ctx.db.player_lowercase_username_state().try_insert(PlayerLowercaseUsernameState {
        entity_id,
        username_lowercase,
    })?;

    ctx.db.satiation_state().try_insert(SatiationState {
        entity_id,
        satiation: SatiationState::get_player_max_satiation(ctx, entity_id),
    })?;

    let num_pockets = ctx
        .db
        .parameters_desc_v2()
        .version()
        .find(&0)
        .unwrap()
        .default_num_inventory_pockets;

    let num_cargo_pockets = 1;

    if !InventoryState::new(
        ctx,
        num_pockets + num_cargo_pockets,
        default_pocket_volume,
        default_pocket_volume, // can only hold 1 cargo at a time
        num_pockets,           // cargo pockets at the end
        entity_id,
        0,
        None,
    ) {
        log::info!("Failed to create player inventory");
        return Err(format!("Failed to create player inventory"));
    };

    // On character creations, hunting and combat actions will be the same (only skills with no requirements)
    let hunting_toolbar = ToolbarState {
        entity_id: game_state::create_entity(ctx),
        owner_entity_id: entity_id,
        index: 0,
        actions: Vec::new(),
    };

    let combat_toolbar = ToolbarState {
        entity_id: game_state::create_entity(ctx),
        owner_entity_id: entity_id,
        index: 1,
        actions: Vec::new(),
    };

    let _ = ctx.db.toolbar_state().try_insert(hunting_toolbar);
    let _ = ctx.db.toolbar_state().try_insert(combat_toolbar);

    // Add a combat state for the player. Even if not in combat, we need to know the last timestamp of his last action so we can't delete it.
    ctx.db.combat_state().try_insert(CombatState {
        entity_id,
        last_attacked_timestamp: 0,
        last_performed_action_entity_id: 0,
        global_cooldown: Some(ActionCooldown {
            timestamp: 0,
            cooldown: 0.0,
        }),
    })?;

    ctx.db.attack_outcome_state().try_insert(AttackOutcomeState::new(entity_id))?;
    ctx.db.extract_outcome_state().try_insert(ExtractOutcomeState {
        entity_id,
        target_entity_id: 0,
        last_timestamp: ctx.timestamp,
        damage: 0,
    })?;

    ctx.db.targetable_state().try_insert(TargetableState::new(entity_id))?;

    ctx.db.onboarding_state().try_insert(OnboardingState {
        entity_id,
        completed_states: vec![],
        current_quests: vec![],
        completed_quests: vec![],
    })?;

    AchievementDesc::discover_eligible(ctx, entity_id);

    PlayerState::collect_stats(ctx, entity_id);

    // set starting health and stamina values (needs to be done after initial equipment & stats collecting)
    let max_health = PlayerState::get_stat(ctx, entity_id, CharacterStatType::MaxHealth);
    let max_stamina = PlayerState::get_stat(ctx, entity_id, CharacterStatType::MaxStamina);
    let max_teleportation_energy = PlayerState::get_stat(ctx, entity_id, CharacterStatType::MaxTeleportationEnergy);

    ctx.db.stamina_state().try_insert(StaminaState {
        entity_id,
        stamina: max_stamina * 0.5, // Starting stamina is 50% of max value
        last_stamina_decrease_timestamp: ctx.timestamp,
    })?;

    ctx.db.teleportation_energy_state().try_insert(TeleportationEnergyState {
        entity_id,
        energy: max_teleportation_energy,
    })?;

    ctx.db.health_state().try_insert(HealthState {
        entity_id,
        health: max_health,
        last_health_decrease_timestamp: ctx.timestamp,
        died_timestamp: 0,
    })?;

    ctx.db.global_search_state().try_insert(GlobalSearchState {
        entity_id,
        found_entity_id: 0,
        found_entity_name: "".into(),
        x: 0,
        z: 0,
        timestamp: ctx.timestamp,
    })?;

    // Innerlight buff
    let mut active_buff_state = ctx.db.active_buff_state().entity_id().find(&entity_id).unwrap();
    let innerlight_buff_duration = ctx.db.parameters_desc_v2().version().find(&0).unwrap().new_user_aggro_immunity;
    active_buff_state.set_innerlight_buff(ctx, innerlight_buff_duration);
    // pause buffs as they will be restarted in sign in
    active_buff_state.pause_all_buffs(ctx);
    ctx.db.active_buff_state().entity_id().update(active_buff_state);

    // Auto attack on slot 0
    player::ability_set::reduce(ctx, entity_id, 0, 0, AbilityType::AutoAttack)?;

    //add sword to toolbelt
    if let Some(mut toolbelt) = InventoryState::get_player_toolbelt(ctx, entity_id) {
        toolbelt.set_at(
            (ctx.db.parameters_desc_v2().version().find(&0).unwrap().default_num_toolbelt_pockets - 1) as usize,
            Some(ItemStack::new(ctx, weapon_equipment_id, ItemType::Item, 1)),
        );

        ctx.db.inventory_state().entity_id().update(toolbelt);
        PlayerState::init_toolbelt(ctx, entity_id, weapon_equipment_id);
        PlayerState::on_added_to_toolbelt(ctx, entity_id, weapon_equipment_id);
    }

    // Eat Mushroom Skewer Action to ease-in tutorial goods
    player::ability_set::reduce(ctx, entity_id, 0, 11, AbilityType::Eat(1170001))?;

    send_inter_module_message(
        ctx,
        crate::messages::inter_module::MessageContentsV4::OnRegionPlayerCreated(OnRegionPlayerCreatedMsg {
            player_entity_id: entity_id,
        }),
        crate::inter_module::InterModuleDestination::Global,
    );

    log::info!("Player created... {}", entity_id);

    Ok(entity_id)
}
