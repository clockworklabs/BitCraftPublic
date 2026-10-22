use std::borrow::Borrow;

use spacetimedb::{log, ReducerContext};

use super::Discovery;
use crate::{
    messages::{
        components::*,
        game_util::ItemType,
        static_data::{crafting_recipe_discovery_item_desc, pillar_shaping_desc},
    },
    *,
};

#[spacetimedb::table(accessor = refresh_player_knowledges_timer, scheduled(refresh_player_knowledges_scheduled, at = scheduled_at))]
pub struct RefreshPlayerKnowledgesTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: spacetimedb::ScheduleAt,
    pub player_entity_ids: Vec<u64>,
}

#[spacetimedb::reducer]
fn refresh_player_knowledges_scheduled(ctx: &ReducerContext, timer: RefreshPlayerKnowledgesTimer) {
    let count = timer.player_entity_ids.len();
    Discovery::refresh_players_knowledges(ctx, timer.player_entity_ids);
    log::info!("Refreshed knowledges for {} players.", count);
}

impl Discovery {
    pub(super) fn knowledge_entry_array_hash(entries: &Vec<KnowledgeEntry>) -> i32 {
        entries.iter().filter(|e| e.state == KnowledgeState::Acquired).count() as i32 * 10000
            + entries.iter().filter(|e| e.state == KnowledgeState::Discovered).count() as i32
    }

    pub(super) fn location_entry_array_hash(entries: &Vec<KnowledgeLocationEntry>) -> i32 {
        entries.iter().filter(|e| e.state == KnowledgeState::Acquired).count() as i32 * 10000
            + entries.iter().filter(|e| e.state == KnowledgeState::Discovered).count() as i32
    }

    #[allow(dead_code)]
    pub(super) fn entity_entry_array_hash(entries: &Vec<KnowledgeEntityEntry>) -> i32 {
        entries.iter().filter(|e| e.state == KnowledgeState::Acquired).count() as i32 * 10000
            + entries.iter().filter(|e| e.state == KnowledgeState::Discovered).count() as i32
    }

    pub(super) fn discover_construction_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.construction_recipe_desc().id().find(&recipe_id) {
            // discover produced building
            self.discover_building(ctx, recipe.building_description_id);
            // discover required items
            for i in &recipe.consumed_item_stacks {
                self.discover_item_and_item_list(ctx, i.item_id);
            }
            // discover required cargos
            for i in &recipe.consumed_cargo_stacks {
                self.discover_cargo(ctx, i.item_id);
            }
        }
    }

    pub(super) fn discover_resource_placement_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.resource_placement_recipe_desc().id().find(&recipe_id) {
            // discover produced resource
            self.discover_resource(ctx, recipe.resource_description_id);
            // discover required items
            for i in &recipe.consumed_item_stacks {
                self.discover_item_and_item_list(ctx, i.item_id);
            }
            // discover required cargos
            for i in &recipe.consumed_cargo_stacks {
                self.discover_cargo(ctx, i.item_id);
            }
        }
    }

    // discover crafting recipe required/comsumed elements
    pub(super) fn discover_craft_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.crafting_recipe_desc().id().find(&recipe_id) {
            // discover produced items
            for i in &recipe.crafted_item_stacks {
                if i.item_type == ItemType::Cargo {
                    self.discover_cargo(ctx, i.item_id);
                } else {
                    self.discover_item_and_item_list(ctx, i.item_id);
                }
            }
            // discover required items and cargos
            for i in &recipe.consumed_item_stacks {
                if i.item_type == ItemType::Item {
                    self.discover_item_and_item_list(ctx, i.item_id);
                } else {
                    self.discover_cargo(ctx, i.item_id);
                }
            }
        }
    }

    // discover construction recipe required/comsumed elements
    pub(super) fn discover_extract_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.extraction_recipe_desc().id().find(&recipe_id) {
            // discover all extracted items
            for i in &recipe.extracted_item_stacks {
                let item_stack = i.item_stack.as_ref().unwrap();
                if item_stack.item_type == ItemType::Cargo {
                    self.discover_cargo(ctx, item_stack.item_id);
                } else {
                    self.discover_item_and_item_list(ctx, item_stack.item_id);
                }
            }
            // discover consumed items/cargos
            for i in &recipe.consumed_item_stacks {
                if i.item_type == ItemType::Cargo {
                    self.discover_cargo(ctx, i.item_id);
                } else {
                    self.discover_item_and_item_list(ctx, i.item_id);
                }
            }
        }
    }

    // discover paving recipe required/comsumed elements (assuming it's discovered by a discovery trigger)
    pub(super) fn discover_paving_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.paving_tile_desc().id().find(&recipe_id) {
            // discover all extracted items
            for i in &recipe.consumed_item_stacks {
                self.discover_item_and_item_list(ctx, i.item_id);
            }
            // discover consumed items/cargos
            if recipe.input_cargo_id != 0 {
                self.discover_cargo(ctx, recipe.input_cargo_id);
            }
        }
    }

    pub(super) fn discover_pillar_shaping_recipe_components(&mut self, ctx: &ReducerContext, recipe_id: i32) {
        if let Some(recipe) = ctx.db.pillar_shaping_desc().id().find(&recipe_id) {
            // discover all extracted items
            for i in &recipe.consumed_item_stacks {
                self.discover_item_and_item_list(ctx, i.item_id);
            }
            // discover consumed items/cargos
            if recipe.input_cargo_id != 0 {
                self.discover_cargo(ctx, recipe.input_cargo_id);
            }
        }
    }

    pub(super) fn on_item_acquired(&mut self, ctx: &ReducerContext, item_id: i32) {
        if let Some(recipe) = ctx.db.item_desc().id().find(&item_id) {
            // acquire secondary knowledge from acquiring an item
            if recipe.secondary_knowledge_id != 0 {
                self.acquire_secondary(ctx, recipe.secondary_knowledge_id);
            }

            self.evaluate_craft_discoveries(
                ctx,
                ctx.db
                    .crafting_recipe_discovery_item_desc()
                    .item_id()
                    .filter(item_id)
                    .map(|r| ctx.db.crafting_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_construction_discoveries(
                ctx,
                ctx.db
                    .construction_recipe_discovery_item_desc()
                    .item_id()
                    .filter(item_id)
                    .map(|r| ctx.db.construction_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_paving_discoveries(
                ctx,
                ctx.db
                    .paving_recipe_discovery_item_desc()
                    .item_id()
                    .filter(item_id)
                    .map(|r| ctx.db.paving_tile_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_resource_placement_discoveries(
                ctx,
                ctx.db
                    .resource_placement_recipe_discovery_item_desc()
                    .item_id()
                    .filter(item_id)
                    .map(|r| ctx.db.resource_placement_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_pillar_shaping_discoveries(
                ctx,
                ctx.db
                    .pillar_shaping_recipe_discovery_item_desc()
                    .item_id()
                    .filter(item_id)
                    .map(|r| ctx.db.pillar_shaping_desc().id().find(r.recipe_id).unwrap()),
            );
        }
    }

    pub(super) fn on_cargo_acquired(&mut self, ctx: &ReducerContext, cargo_id: i32) {
        if let Some(recipe) = ctx.db.cargo_desc().id().find(&cargo_id) {
            // acquire secondary knowledge from acquiring a cargo
            if recipe.secondary_knowledge_id != 0 {
                self.acquire_secondary(ctx, recipe.secondary_knowledge_id);
            }

            self.evaluate_craft_discoveries(
                ctx,
                ctx.db
                    .crafting_recipe_discovery_cargo_desc()
                    .cargo_id()
                    .filter(cargo_id)
                    .map(|r| ctx.db.crafting_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_construction_discoveries(
                ctx,
                ctx.db
                    .construction_recipe_discovery_cargo_desc()
                    .cargo_id()
                    .filter(cargo_id)
                    .map(|r| ctx.db.construction_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_paving_discoveries(
                ctx,
                ctx.db
                    .paving_recipe_discovery_cargo_desc()
                    .cargo_id()
                    .filter(cargo_id)
                    .map(|r| ctx.db.paving_tile_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_resource_placement_discoveries(
                ctx,
                ctx.db
                    .resource_placement_recipe_discovery_cargo_desc()
                    .cargo_id()
                    .filter(cargo_id)
                    .map(|r| ctx.db.resource_placement_recipe_desc().id().find(r.recipe_id).unwrap()),
            );
            self.evaluate_pillar_shaping_discoveries(
                ctx,
                ctx.db
                    .pillar_shaping_recipe_discovery_cargo_desc()
                    .cargo_id()
                    .filter(cargo_id)
                    .map(|r| ctx.db.pillar_shaping_desc().id().find(r.recipe_id).unwrap()),
            );
        }
    }

    pub(super) fn on_secondary_acquired(&mut self, ctx: &ReducerContext, knowledge_id: i32) {
        self.evaluate_craft_discoveries(
            ctx,
            ctx.db
                .crafting_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.crafting_recipe_desc().id().find(r.recipe_id).unwrap()),
        );
        self.evaluate_construction_discoveries(
            ctx,
            ctx.db
                .construction_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.construction_recipe_desc().id().find(r.recipe_id).unwrap()),
        );
        self.evaluate_extract_discoveries(
            ctx,
            ctx.db
                .construction_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.construction_recipe_desc().id().find(r.recipe_id).unwrap()),
        );
        self.evaluate_paving_discoveries(
            ctx,
            ctx.db
                .paving_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.paving_tile_desc().id().find(r.recipe_id).unwrap()),
        );
        self.evaluate_resource_placement_discoveries(
            ctx,
            ctx.db
                .resource_placement_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.resource_placement_recipe_desc().id().find(r.recipe_id).unwrap()),
        );
        self.evaluate_pillar_shaping_discoveries(
            ctx,
            ctx.db
                .pillar_shaping_recipe_discovery_knowledge_desc()
                .knowledge_id()
                .filter(knowledge_id)
                .map(|r| ctx.db.pillar_shaping_desc().id().find(r.recipe_id).unwrap()),
        );
        EquipmentPresetState::on_knowledge_acquired(ctx, self.player_entity_id, knowledge_id);
    }

    fn evaluate_craft_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<CraftingRecipeDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_craft(recipe.id) {
                let mut discover_recipe = false;

                // any discovery trigger overrides all
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        discover_recipe = true;
                    }
                }

                // player is missing mandatory secondary knowledge
                let mut has_all_required_knowledges = true;
                for k_id in &recipe.required_knowledges {
                    has_all_required_knowledges &= self.has_acquired_secondary(*k_id);
                }

                if has_all_required_knowledges {
                    // score all items discoveries
                    let mut max_score = 0;
                    let mut score = 0;
                    for item_stack in &recipe.consumed_item_stacks {
                        if item_stack.discovery_score > 0 {
                            max_score += item_stack.discovery_score;
                            if item_stack.item_type == ItemType::Item {
                                if self.has_acquired_item(item_stack.item_id) {
                                    score += item_stack.discovery_score;
                                }
                            } else {
                                if self.has_acquired_cargo(item_stack.item_id) {
                                    score += item_stack.discovery_score;
                                }
                            }
                        }
                    }

                    let max_value = recipe.full_discovery_score.min(max_score);
                    if max_value > 0 {
                        // discover recipe if enough discovery points is scored (capping to what's possible to get in the columns)
                        if score >= recipe.full_discovery_score.min(max_score) {
                            discover_recipe = true;
                        }
                    }
                }

                if discover_recipe {
                    self.discover_craft(ctx, recipe.id);
                }
            }
        }
    }

    fn evaluate_construction_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<ConstructionRecipeDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_construction(recipe.id) {
                let mut discover_recipe = false;

                // any discovery trigger overrides all
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        discover_recipe = true;
                    }
                }

                // player is missing mandatory secondary knowledge
                let mut has_all_required_knowledges = true;
                for k_id in &recipe.required_knowledges {
                    has_all_required_knowledges &= self.has_acquired_secondary(*k_id);
                }

                if has_all_required_knowledges {
                    // score all items discoveries
                    let mut max_score = 0;
                    let mut score = 0;
                    for item_stack in &recipe.consumed_item_stacks {
                        if item_stack.discovery_score > 0 {
                            max_score += item_stack.discovery_score;
                            if self.has_acquired_item(item_stack.item_id) {
                                score += item_stack.discovery_score;
                            }
                        }
                    }
                    for cargo_stack in &recipe.consumed_cargo_stacks {
                        if cargo_stack.discovery_score > 0 {
                            max_score += cargo_stack.discovery_score;
                            if self.has_acquired_cargo(cargo_stack.item_id) {
                                score += cargo_stack.discovery_score;
                            }
                        }
                    }

                    let req_value = recipe.full_discovery_score.min(max_score);
                    if req_value > 0 {
                        // discover recipe if enough discovery points is scored (capping to what's possible to get in the columns)
                        if score >= req_value {
                            discover_recipe = true;
                        }
                    }
                }

                if discover_recipe {
                    self.discover_construction(ctx, recipe.id);
                }
            }
        }
    }

    fn evaluate_pillar_shaping_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<PillarShapingDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_pillar_shaping(recipe.id) {
                let mut discover_recipe = false;

                // any discovery trigger overrides all
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        discover_recipe = true;
                    }
                }

                // player is missing mandatory secondary knowledge
                let mut has_all_required_knowledges = true;
                for k_id in &recipe.required_knowledges {
                    has_all_required_knowledges &= self.has_acquired_secondary(*k_id);
                }

                if has_all_required_knowledges {
                    // score all items discoveries
                    let mut max_score = 0;
                    let mut score = 0;
                    for item_stack in &recipe.consumed_item_stacks {
                        if item_stack.discovery_score > 0 {
                            max_score += item_stack.discovery_score;
                            if item_stack.item_type == ItemType::Item {
                                if self.has_acquired_item(item_stack.item_id) {
                                    score += item_stack.discovery_score;
                                }
                            } else {
                                if self.has_acquired_cargo(item_stack.item_id) {
                                    score += item_stack.discovery_score;
                                }
                            }
                        }
                    }

                    let req_value = recipe.full_discovery_score.min(max_score);
                    if req_value > 0 {
                        // discover recipe if enough discovery points is scored (capping to what's possible to get in the columns)
                        if score >= req_value {
                            discover_recipe = true;
                        }
                    }
                }

                if discover_recipe {
                    self.discover_pillar_shaping(ctx, recipe.id);
                }
            }
        }
    }

    fn evaluate_resource_placement_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<ResourcePlacementRecipeDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_resource_placement(recipe.id) {
                let mut discover_recipe = false;

                // any discovery trigger overrides all
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        discover_recipe = true;
                    }
                }

                // player is missing mandatory secondary knowledge
                let mut has_all_required_knowledges = true;
                for k_id in &recipe.required_knowledges {
                    has_all_required_knowledges &= self.has_acquired_secondary(*k_id);
                }

                if has_all_required_knowledges {
                    // score all items discoveries
                    let mut max_score = 0;
                    let mut score = 0;
                    for item_stack in &recipe.consumed_item_stacks {
                        if item_stack.discovery_score > 0 {
                            max_score += item_stack.discovery_score;
                            if self.has_acquired_item(item_stack.item_id) {
                                score += item_stack.discovery_score;
                            }
                        }
                    }
                    for cargo_stack in &recipe.consumed_cargo_stacks {
                        if cargo_stack.discovery_score > 0 {
                            max_score += cargo_stack.discovery_score;
                            if self.has_acquired_cargo(cargo_stack.item_id) {
                                score += cargo_stack.discovery_score;
                            }
                        }
                    }

                    let req_value = recipe.full_discovery_score.min(max_score);
                    if req_value > 0 {
                        // discover recipe if enough discovery points is scored (capping to what's possible to get in the columns)
                        if score >= req_value {
                            discover_recipe = true;
                        }
                    }
                }

                if discover_recipe {
                    self.discover_resource_placement(ctx, recipe.id);
                }
            }
        }
    }

    fn evaluate_extract_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<ConstructionRecipeDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_extract(recipe.id) {
                // extraction is a special case. The discovery trigger teaches extraction instead of simply discovering it.
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        self.acquire_extract(ctx, recipe.id);
                        break;
                    }
                }
            }
        }
    }

    fn evaluate_paving_discoveries<I>(&mut self, ctx: &ReducerContext, recipes: I)
    where
        I: IntoIterator,
        I::Item: Borrow<PavingTileDesc>,
    {
        for recipe in recipes {
            let recipe = recipe.borrow();
            if !self.has_discovered_paving(recipe.id) {
                let mut discover_recipe = false;

                // any discovery trigger overrides all
                for dt_id in &recipe.discovery_triggers {
                    if self.has_acquired_secondary(*dt_id) {
                        discover_recipe = true;
                    }
                }

                // player is missing mandatory secondary knowledge
                let mut has_all_required_knowledges = true;
                for k_id in &recipe.required_knowledges {
                    has_all_required_knowledges &= self.has_acquired_secondary(*k_id);
                }

                if has_all_required_knowledges {
                    // score all items discoveries
                    let mut max_score = 0;
                    let mut score = 0;
                    for item_stack in &recipe.consumed_item_stacks {
                        if item_stack.discovery_score > 0 {
                            max_score += item_stack.discovery_score;
                            if self.has_acquired_item(item_stack.item_id) {
                                score += item_stack.discovery_score;
                            }
                        }
                    }
                    if recipe.input_cargo_discovery_score > 0 {
                        max_score += recipe.input_cargo_discovery_score;
                        if self.has_acquired_cargo(recipe.input_cargo_id) {
                            score += recipe.input_cargo_discovery_score;
                        }
                    }

                    let req_value = recipe.full_discovery_score.min(max_score);
                    if req_value > 0 {
                        // discover recipe if enough discovery points is scored (capping to what's possible to get in the columns)
                        if score >= req_value {
                            discover_recipe = true;
                        }
                    }
                }

                if discover_recipe {
                    self.discover_paving(ctx, recipe.id);
                }
            }
        }
    }

    pub fn schedule_all_players_knowledges_refresh(ctx: &ReducerContext) {
        for timer in ctx.db.refresh_player_knowledges_timer().iter() {
            ctx.db.refresh_player_knowledges_timer().scheduled_id().delete(timer.scheduled_id);
        }

        const BATCH_SIZE: usize = 250;
        let mut batch: Vec<u64> = Vec::new();
        let mut count = 0;

        for p in ctx.db.player_state().iter() {
            batch.push(p.entity_id);
            count += 1;

            if batch.len() == BATCH_SIZE {
                ctx.db
                    .refresh_player_knowledges_timer()
                    .try_insert(RefreshPlayerKnowledgesTimer {
                        scheduled_id: 0,
                        scheduled_at: ctx.timestamp.into(),
                        player_entity_ids: batch.clone(),
                    })
                    .unwrap();
                batch.clear();
            }
        }

        ctx.db
            .refresh_player_knowledges_timer()
            .try_insert(RefreshPlayerKnowledgesTimer {
                scheduled_id: 0,
                scheduled_at: ctx.timestamp.into(),
                player_entity_ids: batch,
            })
            .unwrap();

        log::info!("Scheduled knowledge refresh for {} players", count);
    }

    // Refresh all auto-discovered recipes
    pub fn refresh_all_players_knowledges(ctx: &ReducerContext) {
        Self::refresh_players_knowledges(ctx, ctx.db.player_state().iter().map(|x| x.entity_id).collect());
    }

    pub fn refresh_players_knowledges(ctx: &ReducerContext, player_entity_ids: Vec<u64>) {
        let crafting_recipes: Vec<CraftingRecipeDesc> = ctx.db.crafting_recipe_desc().iter().collect();
        let construction_recipes: Vec<ConstructionRecipeDesc> = ctx.db.construction_recipe_desc().iter().collect();
        let resource_placement_recipes: Vec<ResourcePlacementRecipeDesc> = ctx.db.resource_placement_recipe_desc().iter().collect();
        let paving_tile_recipes: Vec<PavingTileDesc> = ctx.db.paving_tile_desc().iter().collect();
        let pillar_shaping_recipes: Vec<PillarShapingDesc> = ctx.db.pillar_shaping_desc().iter().collect();

        for player_entity_id in player_entity_ids {
            let mut discovery = Discovery::new(player_entity_id);
            discovery.initialize(ctx);
            discovery.evaluate_craft_discoveries(ctx, crafting_recipes.iter());
            discovery.evaluate_construction_discoveries(ctx, construction_recipes.iter());
            discovery.evaluate_resource_placement_discoveries(ctx, resource_placement_recipes.iter());
            discovery.evaluate_extract_discoveries(ctx, construction_recipes.iter());
            discovery.evaluate_paving_discoveries(ctx, paving_tile_recipes.iter());
            discovery.evaluate_pillar_shaping_discoveries(ctx, pillar_shaping_recipes.iter());
            discovery.commit(ctx);
        }
    }

    pub fn discover_item_and_item_list(&mut self, ctx: &ReducerContext, item_id: i32) {
        if let Some(item_list) = ctx.db.item_desc().id().find(&item_id) {
            let item_list_id = item_list.item_list_id;
            if item_list_id != 0 {
                let item_list = ctx.db.item_list_desc().id().find(&item_list_id).unwrap();
                for possibility in item_list.possibilities {
                    for item in possibility.items {
                        if item.item_type == ItemType::Item {
                            self.discover_item_and_item_list(ctx, item.item_id);
                        } else {
                            self.discover_cargo(ctx, item.item_id);
                        }
                    }
                }
            }
            self.discover_item(ctx, item_id);
        } else {
            log::error!("Trying to discover unknown item id {}", item_id);
        }
    }
}
