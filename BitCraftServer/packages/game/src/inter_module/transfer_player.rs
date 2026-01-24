use bitcraft_macro::{event_table, shared_table_reducer};
use spacetimedb::{ReducerContext, Table};

use crate::{
    game::{
        coordinates::{region_coordinates::RegionCoordinates, FloatHexTile},
        handlers::{
            player::sign_out::sign_out_internal,
            queue::{end_grace_period::end_grace_period_timer, player_queue},
            server::player_clear_action_state,
        },
        reducer_helpers::deployable_helpers,
    },
    messages::{
        authentication::ServerIdentity,
        components::*,
        empire_shared::{empire_player_data_state, EmpireState},
        generic::world_region_state,
        inter_module::{MessageContentsV4, TransferPlayerMsgV4},
        static_data::BuffCategory,
    },
    unwrap_or_return,
    utils::from_ctx::FromCtx,
};

use super::{send_inter_module_message, user_update_region};

pub fn send_message(ctx: &ReducerContext, entity_id: u64, destination: FloatHexTile, with_vehicle: bool, teleport_energy_cost: f32) {
    //Add client event
    let region = ctx.db.world_region_state().iter().next().unwrap();
    let new_region_index = RegionCoordinates::from_ctx(ctx, destination).to_region_index(region.region_count_sqrt);
    PlayerRegionTransferEvent::new_event(ctx, entity_id, new_region_index);

    //Schedule transfer reducer (it has to be a separate transaction, otherwise clients will delete local players)
    ctx.db.transfer_player_timer().insert(TransferPlayerTimer {
        scheduled_id: 0,
        scheduled_at: ctx.timestamp.into(),
        entity_id,
        destination,
        new_region_index,
        with_vehicle,
        teleport_energy_cost,
    });
}

#[spacetimedb::table(name = transfer_player_timer, scheduled(transfer_player_delayed, at = scheduled_at))]
pub struct TransferPlayerTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub entity_id: u64,
    pub destination: FloatHexTile,
    pub new_region_index: u8,
    pub with_vehicle: bool,
    pub teleport_energy_cost: f32,
}

#[spacetimedb::reducer]
#[shared_table_reducer]
fn transfer_player_delayed(ctx: &ReducerContext, timer: TransferPlayerTimer) {
    if ServerIdentity::validate_server_or_admin(&ctx).is_err() {
        spacetimedb::log::error!("Unauthorized access to transfer_player_delayed");
        return;
    }

    let entity_id = timer.entity_id;
    let destination = timer.destination;
    let new_region_index = timer.new_region_index;

    let mut user_state = unwrap_or_return!(ctx.db.user_state().entity_id().find(entity_id), "Player doesn't exist");
    let mes = ctx.db.mobile_entity_state().entity_id().find(entity_id).unwrap();
    let identity = user_state.identity;

    let mut vehicle = None;
    let mut vehicle_inventory = None;
    if timer.with_vehicle {
        if let Some(mount) = ctx.db.mounting_state().entity_id().find(entity_id) {
            if mount.deployable_slot == 0 {
                deployable_helpers::expel_passengers(ctx, mount.deployable_entity_id, true, false);
                vehicle = ctx.db.deployable_state().entity_id().find(mount.deployable_entity_id);
                vehicle_inventory = ctx.db.inventory_state().owner_entity_id().filter(mount.deployable_entity_id).next();

                ctx.db.deployable_state().delete(vehicle.clone().unwrap());
                ctx.db.inventory_state().delete(vehicle_inventory.clone().unwrap());
                ctx.db.mobile_entity_state().entity_id().delete(mount.deployable_entity_id);
                ctx.db.mounting_state().delete(mount);
            }
        }
    }

    sign_out_internal(ctx, identity, false);
    for row in ctx.db.player_action_state().entity_id().filter(entity_id) {
        let _ = player_clear_action_state::reduce(ctx, row.entity_id, PlayerActionType::None, row.layer, PlayerActionResult::Success);
    }

    sign_out_internal(ctx, identity, false);

    let allow_cancel = ctx.db.user_previous_region_state().identity().find(identity).is_none();
    user_state.can_sign_in = !allow_cancel; //True when returning to prev module (to skip queue), otherwise false

    //Collect all player components into a message
    let mut player_state = ctx.db.player_state().entity_id().find(entity_id).unwrap();
    player_state.signed_in = false;
    let move_validation_strike_counter_state = ctx.db.move_validation_strike_counter_state().entity_id().find(entity_id).unwrap();
    let health_state = ctx.db.health_state().entity_id().find(entity_id).unwrap();
    let stamina_state = ctx.db.stamina_state().entity_id().find(entity_id).unwrap();
    let experience_state = ctx.db.experience_state().entity_id().find(entity_id).unwrap();
    let active_buff_state = ctx.db.active_buff_state().entity_id().find(entity_id).unwrap();
    let knowledge_achievement_state = ctx.db.knowledge_achievement_state().entity_id().find(entity_id).unwrap();
    let knowledge_battle_action_state = ctx.db.knowledge_battle_action_state().entity_id().find(entity_id).unwrap();
    let knowledge_building_state = ctx.db.knowledge_building_state().entity_id().find(entity_id).unwrap();
    let knowledge_cargo_state = ctx.db.knowledge_cargo_state().entity_id().find(entity_id).unwrap();
    let knowledge_construction_state = ctx.db.knowledge_construction_state().entity_id().find(entity_id).unwrap();
    let knowledge_resource_placement_state = ctx.db.knowledge_resource_placement_state().entity_id().find(entity_id).unwrap();
    let knowledge_craft_state = ctx.db.knowledge_craft_state().entity_id().find(entity_id).unwrap();
    let knowledge_enemy_state = ctx.db.knowledge_enemy_state().entity_id().find(entity_id).unwrap();
    let knowledge_extract_state = ctx.db.knowledge_extract_state().entity_id().find(entity_id).unwrap();
    let knowledge_item_state = ctx.db.knowledge_item_state().entity_id().find(entity_id).unwrap();
    let knowledge_lore_state = ctx.db.knowledge_lore_state().entity_id().find(entity_id).unwrap();
    let knowledge_npc_state = ctx.db.knowledge_npc_state().entity_id().find(entity_id).unwrap();
    let knowledge_resource_state = ctx.db.knowledge_resource_state().entity_id().find(entity_id).unwrap();
    let knowledge_ruins_state = ctx.db.knowledge_ruins_state().entity_id().find(entity_id).unwrap();
    let knowledge_secondary_state = ctx.db.knowledge_secondary_state().entity_id().find(entity_id).unwrap();
    let knowledge_vault_state = ctx.db.knowledge_vault_state().entity_id().find(entity_id).unwrap();
    let knowledge_deployable_state = ctx.db.knowledge_deployable_state().entity_id().find(entity_id).unwrap();
    let knowledge_paving_state = ctx.db.knowledge_paving_state().entity_id().find(entity_id).unwrap();
    let knowledge_claim_state = ctx.db.knowledge_claim_state().entity_id().find(entity_id).unwrap();
    let knowledge_pillar_shaping_state = ctx.db.knowledge_pillar_shaping_state().entity_id().find(entity_id).unwrap();
    let equipment_state = ctx.db.equipment_state().entity_id().find(entity_id).unwrap();
    let inventory_state = ctx.db.inventory_state().owner_entity_id().filter(entity_id).collect();
    let character_stats_state = ctx.db.character_stats_state().entity_id().find(entity_id).unwrap();
    let player_username_state = ctx.db.player_username_state().entity_id().find(entity_id).unwrap();
    let player_action_state = ctx.db.player_action_state().entity_id().filter(entity_id).collect();
    let deployable_collectible_state_v2 = ctx
        .db
        .deployable_collectible_state_v2()
        .owner_entity_id()
        .filter(entity_id)
        .collect();
    let combat_state = ctx.db.combat_state().entity_id().find(entity_id).unwrap();
    let action_state = ctx.db.action_state().owner_entity_id().filter(entity_id).collect(); // obsolete soon
    let toolbar_state = ctx.db.toolbar_state().owner_entity_id().filter(entity_id).collect();
    let action_bar_state = ctx.db.action_bar_state().player_entity_id().filter(entity_id).collect();
    let ability_state = ctx.db.ability_state().owner_entity_id().filter(entity_id).collect();
    let attack_outcome_state = ctx.db.attack_outcome_state().entity_id().find(entity_id).unwrap();
    let vault_state = ctx.db.vault_state().entity_id().find(entity_id).unwrap();
    let exploration_chunks_state = ctx.db.exploration_chunks_state().entity_id().find(entity_id).unwrap();
    let satiation_state = ctx.db.satiation_state().entity_id().find(entity_id).unwrap();
    let player_prefs_state = ctx.db.player_prefs_state().entity_id().find(entity_id).unwrap();
    let onboarding_state = ctx.db.onboarding_state().entity_id().find(entity_id).unwrap();
    let unclaimed_collectibles_state = ctx.db.unclaimed_collectibles_state().identity().find(identity);
    let teleportation_energy_state = ctx.db.teleportation_energy_state().entity_id().find(entity_id).unwrap();
    //let player_housing_state = ctx.db.player_housing_state().entity_id().find(entity_id);
    let traveler_task_states = ctx.db.traveler_task_state().player_entity_id().filter(entity_id).collect();
    let extract_outcome_state = ctx.db.extract_outcome_state().entity_id().find(entity_id).unwrap();
    let undeployed_deployable_states = ctx
        .db
        .deployable_state()
        .owner_id()
        .filter(entity_id)
        .filter(|d| ctx.db.mobile_entity_state().entity_id().find(d.entity_id).is_none())
        .collect();
    let player_settings_state = ctx.db.player_settings_state_v2().entity_id().find(entity_id);
    let quest_chain_states = ctx.db.quest_chain_state().player_entity_id().filter(entity_id).collect();
    //Don't forget to delete these components below

    let msg = TransferPlayerMsgV4 {
        original_location: mes.coordinates_float(),
        destination_location: destination,
        allow_cancel,
        teleport_energy_cost: timer.teleport_energy_cost,

        vehicle,
        vehicle_inventory,

        player_state,
        user_state,
        move_validation_strike_counter_state,
        health_state,
        stamina_state,
        experience_state,
        active_buff_state,
        knowledge_achievement_state,
        knowledge_battle_action_state,
        knowledge_building_state,
        knowledge_cargo_state,
        knowledge_construction_state,
        knowledge_resource_placement_state,
        knowledge_craft_state,
        knowledge_enemy_state,
        knowledge_extract_state,
        knowledge_item_state,
        knowledge_lore_state,
        knowledge_npc_state,
        knowledge_resource_state,
        knowledge_ruins_state,
        knowledge_secondary_state,
        knowledge_vault_state,
        knowledge_deployable_state,
        knowledge_paving_state,
        knowledge_claim_state,
        knowledge_pillar_shaping_state,
        equipment_state,
        inventory_state,
        character_stats_state,
        player_username_state,
        player_action_state,
        deployable_collectible_state_v2,
        combat_state,
        action_state,
        toolbar_state,
        action_bar_state,
        ability_state,
        attack_outcome_state,
        vault_state,
        exploration_chunks_state,
        satiation_state,
        player_prefs_state,
        onboarding_state,
        unclaimed_collectibles_state,
        teleportation_energy_state,
        player_housing_state: None, //Housing is replicated to global module now
        traveler_task_states,
        extract_outcome_state,
        undeployed_deployable_states,
        player_settings_state,
        quest_chain_states,
    };
    send_inter_module_message(
        ctx,
        MessageContentsV4::TransferPlayerRequest(msg),
        super::InterModuleDestination::Region(new_region_index),
    );

    //Delete player-related components that don't need to be transfered
    ctx.db.mobile_entity_state().entity_id().delete(entity_id);
    ctx.db.player_lowercase_username_state().entity_id().delete(entity_id);
    ctx.db.mounting_state().entity_id().delete(entity_id);
    ctx.db.target_state().entity_id().delete(entity_id);
    ctx.db.threat_state().entity_id().delete(entity_id);
    ctx.db.targetable_state().entity_id().delete(entity_id);
    //ctx.db.player_vote_state().entity_id().delete(entity_id);
    //ctx.db.alert_state().entity_id().delete(entity_id);
    ctx.db.signed_in_player_state().entity_id().delete(entity_id);
    ctx.db.starving_player_state().entity_id().delete(entity_id);
    ctx.db.end_grace_period_timer().identity().delete(&identity);

    //Delete transfered components
    ctx.db.player_state().entity_id().delete(entity_id);
    ctx.db.user_state().entity_id().delete(entity_id);
    ctx.db.move_validation_strike_counter_state().entity_id().delete(entity_id);
    ctx.db.health_state().entity_id().delete(entity_id);
    ctx.db.user_moderation_state().entity_id().delete(entity_id);
    ctx.db.stamina_state().entity_id().delete(entity_id);
    ctx.db.experience_state().entity_id().delete(entity_id);
    ctx.db.active_buff_state().entity_id().delete(entity_id);
    ctx.db.knowledge_achievement_state().entity_id().delete(entity_id);
    ctx.db.knowledge_battle_action_state().entity_id().delete(entity_id);
    ctx.db.knowledge_building_state().entity_id().delete(entity_id);
    ctx.db.knowledge_cargo_state().entity_id().delete(entity_id);
    ctx.db.knowledge_construction_state().entity_id().delete(entity_id);
    ctx.db.knowledge_resource_placement_state().entity_id().delete(entity_id);
    ctx.db.knowledge_craft_state().entity_id().delete(entity_id);
    ctx.db.knowledge_enemy_state().entity_id().delete(entity_id);
    ctx.db.knowledge_extract_state().entity_id().delete(entity_id);
    ctx.db.knowledge_item_state().entity_id().delete(entity_id);
    ctx.db.knowledge_lore_state().entity_id().delete(entity_id);
    ctx.db.knowledge_npc_state().entity_id().delete(entity_id);
    ctx.db.knowledge_resource_state().entity_id().delete(entity_id);
    ctx.db.knowledge_ruins_state().entity_id().delete(entity_id);
    ctx.db.knowledge_secondary_state().entity_id().delete(entity_id);
    ctx.db.knowledge_vault_state().entity_id().delete(entity_id);
    ctx.db.knowledge_deployable_state().entity_id().delete(entity_id);
    ctx.db.knowledge_paving_state().entity_id().delete(entity_id);
    ctx.db.knowledge_claim_state().entity_id().delete(entity_id);
    ctx.db.knowledge_pillar_shaping_state().entity_id().delete(entity_id);
    ctx.db.equipment_state().entity_id().delete(entity_id);
    ctx.db.inventory_state().owner_entity_id().delete(entity_id);
    ctx.db.character_stats_state().entity_id().delete(entity_id);
    ctx.db.player_username_state().entity_id().delete(entity_id);
    ctx.db.player_action_state().entity_id().delete(entity_id);
    ctx.db.deployable_collectible_state_v2().owner_entity_id().delete(entity_id);
    ctx.db.combat_state().entity_id().delete(entity_id);
    ctx.db.action_state().owner_entity_id().delete(entity_id); // obsolete soon
    ctx.db.toolbar_state().owner_entity_id().delete(entity_id);
    ctx.db.ability_state().owner_entity_id().delete(entity_id);
    ctx.db.action_bar_state().player_entity_id().delete(entity_id);
    ctx.db.attack_outcome_state().entity_id().delete(entity_id);
    ctx.db.vault_state().entity_id().delete(entity_id);
    ctx.db.exploration_chunks_state().entity_id().delete(entity_id);
    ctx.db.satiation_state().entity_id().delete(entity_id);
    ctx.db.player_prefs_state().entity_id().delete(entity_id);
    ctx.db.onboarding_state().entity_id().delete(entity_id);
    ctx.db.unclaimed_collectibles_state().identity().delete(identity);
    ctx.db.player_timestamp_state().entity_id().delete(entity_id);
    ctx.db.teleportation_energy_state().entity_id().delete(entity_id);
    //ctx.db.player_housing_state().entity_id().delete(entity_id);
    ctx.db.traveler_task_state().player_entity_id().delete(entity_id);
    ctx.db.extract_outcome_state().entity_id().delete(entity_id);
    for d in ctx
        .db
        .deployable_state()
        .owner_id()
        .filter(entity_id)
        .filter(|d| ctx.db.mobile_entity_state().entity_id().find(d.entity_id).is_none())
    {
        ctx.db.deployable_state().entity_id().delete(d.entity_id);
    }
    ctx.db.rez_sick_long_term_state().entity_id().delete(entity_id);
    ctx.db.player_settings_state_v2().entity_id().delete(entity_id);
    ctx.db.quest_chain_state().player_entity_id().delete(entity_id);

    player_queue::process_queue(ctx);
}

pub fn process_message_on_destination(ctx: &ReducerContext, _sender: u8, mut msg: TransferPlayerMsgV4) -> Result<(), String> {
    let loc = msg.destination_location.clone();
    let prev_loc = msg.original_location.clone();
    let identity = msg.user_state.identity;
    let allow_cancel = msg.allow_cancel;
    let with_vehicle = msg.vehicle.is_some();
    let teleport_energy_cost = msg.teleport_energy_cost;
    if teleport_energy_cost > 0.0 {
        msg.teleportation_energy_state.expend_energy(teleport_energy_cost, false);
    }

    insert_player(ctx, msg, loc, prev_loc);

    if allow_cancel {
        let prev = UserPreviousRegionState {
            identity,
            previous_region_location: prev_loc,
            with_vehicle,
            allow_cancel,
            teleport_energy_cost,
        };
        if ctx.db.user_previous_region_state().try_insert(prev.clone()).is_err() {
            ctx.db.user_previous_region_state().identity().update(prev);
        } //Upsert sort of
    }

    return user_update_region::send_message(ctx, identity);
}

pub fn handle_destination_result_on_sender(ctx: &ReducerContext, request: TransferPlayerMsgV4, error: Option<String>) {
    if error.is_some() {
        let loc = request.original_location.clone();
        insert_player(ctx, request, loc.clone(), loc);
    }
}

fn insert_player(ctx: &ReducerContext, req: TransferPlayerMsgV4, location: FloatHexTile, previous_location: FloatHexTile) {
    let entity_id = req.user_state.entity_id;
    let name = req.player_username_state.username.clone();
    let satiation = req.satiation_state.satiation;
    let active_buff_state = req.active_buff_state.clone();

    //Insert transfered components
    ctx.db.player_state().insert(req.player_state);
    ctx.db.user_state().insert(req.user_state);
    ctx.db
        .move_validation_strike_counter_state()
        .insert(req.move_validation_strike_counter_state);
    ctx.db.health_state().insert(req.health_state);
    ctx.db.stamina_state().insert(req.stamina_state);
    ctx.db.experience_state().insert(req.experience_state);
    ctx.db.active_buff_state().insert(req.active_buff_state);
    ctx.db.knowledge_achievement_state().insert(req.knowledge_achievement_state);
    ctx.db.knowledge_battle_action_state().insert(req.knowledge_battle_action_state);
    ctx.db.knowledge_building_state().insert(req.knowledge_building_state);
    ctx.db.knowledge_cargo_state().insert(req.knowledge_cargo_state);
    ctx.db.knowledge_construction_state().insert(req.knowledge_construction_state);
    ctx.db
        .knowledge_resource_placement_state()
        .insert(req.knowledge_resource_placement_state);
    ctx.db.knowledge_craft_state().insert(req.knowledge_craft_state);
    ctx.db.knowledge_enemy_state().insert(req.knowledge_enemy_state);
    ctx.db.knowledge_extract_state().insert(req.knowledge_extract_state);
    ctx.db.knowledge_item_state().insert(req.knowledge_item_state);
    ctx.db.knowledge_lore_state().insert(req.knowledge_lore_state);
    ctx.db.knowledge_npc_state().insert(req.knowledge_npc_state);
    ctx.db.knowledge_resource_state().insert(req.knowledge_resource_state);
    ctx.db.knowledge_ruins_state().insert(req.knowledge_ruins_state);
    ctx.db.knowledge_secondary_state().insert(req.knowledge_secondary_state);
    ctx.db.knowledge_vault_state().insert(req.knowledge_vault_state);
    ctx.db.knowledge_deployable_state().insert(req.knowledge_deployable_state);
    ctx.db.knowledge_paving_state().insert(req.knowledge_paving_state);
    ctx.db.knowledge_claim_state().insert(req.knowledge_claim_state);
    ctx.db.knowledge_pillar_shaping_state().insert(req.knowledge_pillar_shaping_state);
    ctx.db.equipment_state().insert(req.equipment_state);
    for i in req.inventory_state {
        ctx.db.inventory_state().insert(i);
    }
    ctx.db.character_stats_state().insert(req.character_stats_state);
    ctx.db.player_username_state().insert(req.player_username_state);
    for mut i in req.player_action_state {
        i.auto_id = 0;
        ctx.db.player_action_state().insert(i);
    }
    for i in req.deployable_collectible_state_v2 {
        ctx.db.deployable_collectible_state_v2().insert(i);
    }
    ctx.db.combat_state().insert(req.combat_state);
    for i in req.action_state {
        // obsolete soon
        ctx.db.action_state().insert(i);
    }

    for i in req.toolbar_state {
        ctx.db.toolbar_state().insert(i);
    }

    for i in req.ability_state {
        ctx.db.ability_state().insert(i);
    }

    for i in req.action_bar_state {
        ctx.db.action_bar_state().insert(i);
    }

    ctx.db.attack_outcome_state().insert(req.attack_outcome_state);
    ctx.db.vault_state().insert(req.vault_state);
    ctx.db.exploration_chunks_state().insert(req.exploration_chunks_state);
    ctx.db.satiation_state().insert(req.satiation_state);
    ctx.db.player_prefs_state().insert(req.player_prefs_state);
    ctx.db.onboarding_state().insert(req.onboarding_state);
    if let Some(val) = req.unclaimed_collectibles_state {
        ctx.db.unclaimed_collectibles_state().insert(val);
    }
    ctx.db.teleportation_energy_state().insert(req.teleportation_energy_state);
    if let Some(val) = req.player_housing_state {
        ctx.db.player_housing_state().insert(val);
    }

    if let Some(vehicle) = req.vehicle {
        let deployable_entity_id = vehicle.entity_id;
        ctx.db.deployable_state().insert(vehicle);
        if let Some(inv) = req.vehicle_inventory {
            ctx.db.inventory_state().insert(inv);
        }

        ctx.db.mounting_state().insert(MountingState {
            entity_id,
            deployable_entity_id,
            deployable_slot: 0,
        });
        ctx.db.mobile_entity_state().insert(MobileEntityState::for_location(
            deployable_entity_id,
            location.into(),
            ctx.timestamp,
        ));
    }
    for i in req.traveler_task_states {
        ctx.db.traveler_task_state().insert(i);
    }
    ctx.db.extract_outcome_state().insert(req.extract_outcome_state);
    for i in req.undeployed_deployable_states {
        ctx.db.deployable_state().insert(i);
    }

    if let Some(player_settings_state) = req.player_settings_state {
        ctx.db.player_settings_state_v2().insert(player_settings_state);
    }

    for i in req.quest_chain_states {
        ctx.db.quest_chain_state().insert(i);
    }

    //Insert components that don't need to be transfered
    ctx.db
        .mobile_entity_state()
        .insert(MobileEntityState::for_location(entity_id, location.into(), ctx.timestamp));

    ctx.db.player_lowercase_username_state().insert(PlayerLowercaseUsernameState {
        entity_id,
        username_lowercase: name.to_lowercase(),
    });
    ctx.db.targetable_state().insert(TargetableState { entity_id });
    if satiation <= 0.0 {
        ctx.db.starving_player_state().insert(StarvingPlayerState { entity_id });
    }

    if active_buff_state
        .active_buff_of_category(ctx, BuffCategory::RezSicknessLongTerm)
        .is_some()
    {
        ctx.db.rez_sick_long_term_state().insert(RezSickLongTermState { entity_id });
    }

    _ = PlayerState::move_player_and_explore_unsafe(ctx, entity_id, &previous_location, &location, 0.0, false, None);
    _ = PlayerState::move_player_and_explore_unsafe(ctx, entity_id, &location, &location, 0.0, false, None);

    //Update emperor crown after transfer (in case it was supposed to be updated mid-transfer
    if let Some(rank) = ctx.db.empire_player_data_state().entity_id().find(entity_id) {
        if rank.rank == 0 {
            _ = EmpireState::update_crown_status(ctx, rank.empire_entity_id);
        }
    }
}

#[event_table(name = player_region_transfer_event)]
pub struct PlayerRegionTransferEvent {
    pub player_entity_id: u64,
    pub new_region_index: u8,
}
