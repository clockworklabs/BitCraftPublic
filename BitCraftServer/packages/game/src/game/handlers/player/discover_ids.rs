use bitcraft_macro::feature_gate;
use spacetimedb::ReducerContext;

use crate::{
    game::{coordinates::OffsetCoordinatesSmall, discovery::Discovery, game_state},
    messages::{components::*, static_data::*},
    unwrap_or_err,
};

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_achievement(ctx: &ReducerContext, achievement_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.achievement_desc().id().find(achievement_id), "Unknown achievement");
    discovery.discover_achievement(ctx, achievement_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_building(ctx: &ReducerContext, building_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.building_desc().id().find(building_id), "Unknown building");
    discovery.discover_building(ctx, building_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_cargo(ctx: &ReducerContext, cargo_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.cargo_desc().id().find(cargo_id), "Unknown cargo");
    discovery.discover_cargo(ctx, cargo_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_claim(ctx: &ReducerContext, claim_entity_id: u64) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.knowledge_claim_state().entity_id().find(claim_entity_id), "Unknown claim");
    discovery.discover_claim(ctx, claim_entity_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_construction_recipe(ctx: &ReducerContext, construction_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(
        ctx.db.construction_recipe_desc().id().find(construction_id),
        "Unknown construction recipe"
    );
    discovery.discover_construction(ctx, construction_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_crafting_recipe(ctx: &ReducerContext, craft_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.crafting_recipe_desc().id().find(craft_id), "Unknown crafting recipe");
    discovery.discover_craft(ctx, craft_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_deployable(ctx: &ReducerContext, deployable_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.deployable_desc().id().find(deployable_id), "Unknown deployable");
    discovery.discover_deployable(ctx, deployable_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_enemy(ctx: &ReducerContext, enemy_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.enemy_desc().enemy_type().find(enemy_id), "Unknown enemy");
    discovery.discover_enemy(ctx, enemy_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_extraction_recipe(ctx: &ReducerContext, extract_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.extraction_recipe_desc().id().find(extract_id), "Unknown extraction recipe");
    discovery.discover_extract(ctx, extract_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_item(ctx: &ReducerContext, item_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.item_desc().id().find(item_id), "Unknown item");
    discovery.discover_item(ctx, item_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_scroll(ctx: &ReducerContext, scroll_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.knowledge_scroll_desc().item_id().find(scroll_id), "Unknown scroll");
    discovery.discover_lore(ctx, scroll_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_npc(ctx: &ReducerContext, npc_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.npc_desc().npc_type().find(npc_id), "Unknown traveler");
    discovery.discover_npc(ctx, npc_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_pavement(ctx: &ReducerContext, paving_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.paving_tile_desc().id().find(paving_id), "Unknown pavement");
    discovery.discover_paving(ctx, paving_id);

    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_pillar_shaping(ctx: &ReducerContext, pillar_shaping_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.pillar_shaping_desc().id().find(pillar_shaping_id), "Unknown pillar shaping");
    discovery.discover_pillar_shaping(ctx, pillar_shaping_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_resource(ctx: &ReducerContext, resource_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.resource_desc().id().find(resource_id), "Unknown resource");
    discovery.discover_resource(ctx, resource_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_resource_placement(ctx: &ReducerContext, resource_placement_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(
        ctx.db.resource_placement_recipe_desc().id().find(resource_placement_id),
        "Unknown resource placement recipe"
    );
    discovery.discover_resource_placement(ctx, resource_placement_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_ruins(ctx: &ReducerContext, coordinates: OffsetCoordinatesSmall) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);
    discovery.discover_ruins(ctx, coordinates);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_secondary_knowledge(ctx: &ReducerContext, secondary_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(
        ctx.db.secondary_knowledge_desc().id().find(secondary_id),
        "Unknown secondary knowledge"
    );
    discovery.discover_secondary(ctx, secondary_id);
    discovery.commit(ctx);
    Ok(())
}

#[spacetimedb::reducer]
#[feature_gate]
pub fn discover_collectible(ctx: &ReducerContext, collectible_id: i32) -> Result<(), String> {
    let actor_id = game_state::actor_id(ctx, true)?;
    PlayerTimestampState::refresh(ctx, actor_id, ctx.timestamp);

    let mut discovery = Discovery::new(actor_id);

    let _ = unwrap_or_err!(ctx.db.collectible_desc().id().find(collectible_id), "Unknown collectible");
    discovery.discover_vault(ctx, collectible_id);
    discovery.commit(ctx);
    Ok(())
}
