use bitcraft_macro::shared_table_reducer;
use spacetimedb::{log, ReducerContext, Table};

use crate::{
    game::{game_state, handlers::authentication::has_role},
    messages::{
        authentication::Role,
        empire_schema::{empire_emblem_state, empire_log_state, EmpireEmblemState, EmpireLogState},
        empire_shared::*,
        static_data::{empire_color_desc, empire_icon_desc},
        util::OffsetCoordinatesSmallMessage,
    },
};

pub const NPC_EMPIRE_DEFAULT_NAME: &str = "Ancient Dominion";

/// Gets the NPC empire entity ID, or creates the NPC empire if it doesn't exist.
/// Creates EmpireState, EmpireEmblemState, and EmpireLogState (to prevent panics in siege notifications).
/// The emblem is always updated to the supplied values so the caller can change colors by re-running.
/// Any ID that is 0 falls back to the first available entry from the corresponding static data table.
pub fn get_or_create_npc_empire(ctx: &ReducerContext, name: &str, icon_id: i32, shape_id: i32, color1_id: i32, color2_id: i32) -> u64 {
    // Resolve 0-valued IDs to first available from static data
    let icon_id = if icon_id == 0 {
        ctx.db.empire_icon_desc().iter().find(|i| !i.is_shape).map(|i| i.id).unwrap_or(0)
    } else {
        icon_id
    };

    let shape_id = if shape_id == 0 {
        ctx.db.empire_icon_desc().iter().find(|i| i.is_shape).map(|i| i.id).unwrap_or(0)
    } else {
        shape_id
    };

    let (color1_id, color2_id) = if color1_id == 0 || color2_id == 0 {
        let mut colors = ctx.db.empire_color_desc().iter();
        let c1 = colors.next().map(|c| c.id).unwrap_or(0);
        let c2 = colors.next().map(|c| c.id).unwrap_or(c1);
        (
            if color1_id == 0 { c1 } else { color1_id },
            if color2_id == 0 { c2 } else { color2_id },
        )
    } else {
        (color1_id, color2_id)
    };

    // Check if NPC empire already exists by owner type (name may have been renamed)
    if let Some(empire) = ctx.db.empire_state().iter().find(|e| e.owner_type == EmpireOwnerType::Npc) {
        let entity_id = empire.entity_id;

        // Update name if it changed
        if empire.name != name {
            let mut updated = empire;
            updated.name = name.to_string();
            EmpireState::update_shared(ctx, updated, crate::inter_module::InterModuleDestination::AllOtherRegions);
            if let Some(mut lower) = ctx.db.empire_lowercase_name_state().entity_id().find(entity_id) {
                lower.name_lowercase = name.to_lowercase();
                ctx.db.empire_lowercase_name_state().entity_id().update(lower);
            }
        }

        // Always upsert emblem so re-running with different colors takes effect
        let emblem = EmpireEmblemState {
            entity_id,
            icon_id,
            shape_id,
            color1_id,
            color2_id,
        };
        if ctx.db.empire_emblem_state().entity_id().find(entity_id).is_some() {
            ctx.db.empire_emblem_state().entity_id().update(emblem);
        } else {
            ctx.db.empire_emblem_state().insert(emblem);
        }
        // Ensure log state exists
        ctx.db
            .empire_log_state()
            .try_insert(EmpireLogState {
                entity_id,
                last_posted: 0,
            })
            .ok();
        // Ensure lowercase name state exists (required for admin_rename_empire_entity)
        ctx.db
            .empire_lowercase_name_state()
            .try_insert(EmpireLowercaseNameState {
                entity_id,
                name_lowercase: name.to_lowercase(),
            })
            .ok();
        return entity_id;
    }

    let entity_id = game_state::create_entity(ctx);

    EmpireState::insert_shared(
        ctx,
        EmpireState {
            entity_id,
            capital_building_entity_id: 0,
            name: name.to_string(),
            shard_treasury: 0,
            empire_currency_treasury: 0,
            nobility_threshold: 0,
            num_claims: 0,
            location: OffsetCoordinatesSmallMessage::default(),
            owner_type: EmpireOwnerType::Npc,
        },
        crate::inter_module::InterModuleDestination::AllOtherRegions,
    );

    ctx.db.empire_emblem_state().insert(EmpireEmblemState {
        entity_id,
        icon_id,
        shape_id,
        color1_id,
        color2_id,
    });

    // Required to prevent unwrap() panic in EmpireNotificationState::new() during siege resolution
    ctx.db
        .empire_log_state()
        .try_insert(EmpireLogState { entity_id, last_posted: 0 })
        .ok();

    // Required for admin_rename_empire_entity
    ctx.db
        .empire_lowercase_name_state()
        .try_insert(EmpireLowercaseNameState {
            entity_id,
            name_lowercase: name.to_lowercase(),
        })
        .ok();

    log::info!("Created NPC empire '{}' with entity_id {}", name, entity_id);
    entity_id
}

/// Gets the NPC empire entity ID, returning an error if it doesn't exist.
/// Looks up by owner_type rather than name so it works regardless of what name was chosen.
pub fn get_npc_empire_id(ctx: &ReducerContext) -> Result<u64, String> {
    ctx.db
        .empire_state()
        .iter()
        .find(|e| e.owner_type == EmpireOwnerType::Npc)
        .map(|e| e.entity_id)
        .ok_or_else(|| "NPC empire does not exist. Call world_form_npc_empire first.".to_string())
}

/// Returns true if the given empire entity ID belongs to an NPC empire.
pub fn is_npc_empire(ctx: &ReducerContext, empire_entity_id: u64) -> bool {
    ctx.db
        .empire_state()
        .entity_id()
        .find(empire_entity_id)
        .map(|e| e.owner_type == EmpireOwnerType::Npc)
        .unwrap_or(false)
}

/// Reducer to delete the NPC empire and all its global state (nodes, chunks, sieges).
/// Building entities are NOT deleted here — the client must call
/// world_clear_npc_watchtowers on each region with the building entity IDs.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn world_clear_npc_empire(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let npc_empire_id = match get_npc_empire_id(ctx) {
        Ok(id) => id,
        Err(_) => {
            log::info!("NPC empire doesn't exist, nothing to clear");
            return Ok(());
        }
    };

    let nodes: Vec<EmpireNodeState> = ctx.db.empire_node_state().empire_entity_id().filter(npc_empire_id).collect();
    let node_count = nodes.len();
    let mut chunk_count = 0u64;

    for node in nodes {
        // Delete chunks assigned to this watchtower
        let chunks: Vec<EmpireChunkState> = ctx.db.empire_chunk_state().watchtower_entity_id().filter(node.entity_id).collect();
        for chunk in chunks {
            EmpireChunkState::delete_shared(ctx, chunk, crate::inter_module::InterModuleDestination::AllOtherRegions);
            chunk_count += 1;
        }

        // Delete sieges on this watchtower
        let sieges: Vec<EmpireNodeSiegeState> = ctx
            .db
            .empire_node_siege_state()
            .building_entity_id()
            .filter(node.entity_id)
            .collect();
        for siege in sieges {
            EmpireNodeSiegeState::delete_shared(ctx, siege, crate::inter_module::InterModuleDestination::AllOtherRegions);
        }

        // Delete the empire node
        EmpireNodeState::delete_shared(ctx, node, crate::inter_module::InterModuleDestination::AllOtherRegions);
    }

    // Delete the empire itself
    if let Some(empire) = ctx.db.empire_state().entity_id().find(npc_empire_id) {
        let entity_id = empire.entity_id;
        EmpireState::delete_shared(ctx, empire, crate::inter_module::InterModuleDestination::AllOtherRegions);
        ctx.db.empire_lowercase_name_state().entity_id().delete(entity_id);
        ctx.db.empire_log_state().entity_id().delete(entity_id);
        ctx.db.empire_emblem_state().entity_id().delete(entity_id);
    }

    log::info!("Cleared NPC empire: {} nodes, {} chunks", node_count, chunk_count);
    Ok(())
}

/// Admin reducer to change an empire's emblem (icon, shape, colors).
/// Mirrors empire_change_emblem but with admin permission check instead of emperor role.
#[spacetimedb::reducer]
pub fn admin_change_empire_emblem(
    ctx: &ReducerContext,
    empire_entity_id: u64,
    icon_id: i32,
    shape_id: i32,
    color1_id: i32,
    color2_id: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut emblem = ctx
        .db
        .empire_emblem_state()
        .entity_id()
        .find(empire_entity_id)
        .ok_or_else(|| format!("Empire emblem not found for entity_id {{0}}|~{}", empire_entity_id))?;

    emblem.icon_id = icon_id;
    emblem.shape_id = shape_id;
    emblem.color1_id = color1_id;
    emblem.color2_id = color2_id;

    ctx.db.empire_emblem_state().entity_id().update(emblem);
    log::info!(
        "Admin changed emblem for empire {}: icon={}, shape={}, color1={}, color2={}",
        empire_entity_id,
        icon_id,
        shape_id,
        color1_id,
        color2_id
    );
    Ok(())
}

/// Admin reducer to assign chunks to a watchtower's territory.
/// Looks up the watchtower's empire_entity_id and upserts EmpireChunkState for each chunk.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_assign_empire_chunks(ctx: &ReducerContext, chunk_indexes: Vec<u64>, watchtower_entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let node = ctx
        .db
        .empire_node_state()
        .entity_id()
        .find(watchtower_entity_id)
        .ok_or_else(|| format!("Watchtower node {{0}} not found|~{}", watchtower_entity_id))?;

    let empire_entity_id = node.empire_entity_id;
    let mut count = 0u64;

    for chunk_index in chunk_indexes {
        if let Some(existing) = ctx.db.empire_chunk_state().chunk_index().find(chunk_index) {
            // Update existing chunk
            let mut updated = existing;
            updated.empire_entity_id = empire_entity_id;
            updated.watchtower_entity_id = watchtower_entity_id;
            EmpireChunkState::update_shared(ctx, updated, crate::inter_module::InterModuleDestination::AllOtherRegions);
        } else {
            // Insert new chunk
            EmpireChunkState::insert_shared(
                ctx,
                EmpireChunkState {
                    chunk_index,
                    empire_entity_id,
                    watchtower_entity_id,
                },
                crate::inter_module::InterModuleDestination::AllOtherRegions,
            );
        }
        count += 1;
    }

    log::info!(
        "Admin assigned {} chunks to watchtower {} (empire {})",
        count,
        watchtower_entity_id,
        empire_entity_id
    );
    Ok(())
}

/// Admin reducer to unassign chunks from empire/watchtower ownership.
/// Sets empire_entity_id and watchtower_entity_id to 0, which triggers deletion
/// via the apply_transaction pattern (chunks with both IDs = 0 are deleted).
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn admin_unassign_empire_chunks(ctx: &ReducerContext, chunk_indexes: Vec<u64>) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut count = 0u64;
    for chunk_index in chunk_indexes {
        if let Some(chunk) = ctx.db.empire_chunk_state().chunk_index().find(chunk_index) {
            EmpireChunkState::delete_shared(ctx, chunk, crate::inter_module::InterModuleDestination::AllOtherRegions);
            count += 1;
        }
    }

    log::info!("Admin unassigned {} chunks from empire ownership", count);
    Ok(())
}

/// Reducer to form/ensure the NPC empire exists on the global module.
/// Must be called before placing NPC watchtowers on regions.
/// Accepts emblem parameters (icon, shape, colors) so the client can pick valid IDs
/// from the EmpireColorDesc static data table.
#[spacetimedb::reducer]
#[shared_table_reducer]
pub fn world_form_npc_empire(
    ctx: &ReducerContext,
    name: String,
    icon_id: i32,
    shape_id: i32,
    color1_id: i32,
    color2_id: i32,
) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Unauthorized".into());
    }

    let empire_name = if name.trim().is_empty() {
        NPC_EMPIRE_DEFAULT_NAME.to_string()
    } else {
        name
    };

    // Validate color IDs against the static data table (same check as empire_form)
    if ctx.db.empire_color_desc().id().find(color1_id).is_none() || ctx.db.empire_color_desc().id().find(color2_id).is_none() {
        return Err(format!(
            "Invalid empire colors: color1_id={{0}}, color2_id={{1}}. Must be valid EmpireColorDesc IDs.|~{}|~{}",
            color1_id, color2_id
        ));
    }

    let entity_id = get_or_create_npc_empire(ctx, &empire_name, icon_id, shape_id, color1_id, color2_id);
    log::info!(
        "NPC empire '{}' formed/verified with entity_id {} (icon={}, shape={}, color1={}, color2={})",
        empire_name,
        entity_id,
        icon_id,
        shape_id,
        color1_id,
        color2_id
    );
    Ok(())
}
