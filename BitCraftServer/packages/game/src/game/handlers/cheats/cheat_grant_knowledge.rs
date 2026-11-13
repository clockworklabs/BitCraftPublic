use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};
use crate::messages::static_data::pillar_shaping_desc;
use crate::{
    building_desc, cargo_desc, collectible_desc, construction_recipe_desc, crafting_recipe_desc, deployable_desc, enemy_desc,
    extraction_recipe_desc, item_desc, knowledge_scroll_desc, npc_desc, paving_tile_desc, resource_desc, resource_placement_recipe_desc,
    secondary_knowledge_desc,
};
use spacetimedb::{log, ReducerContext, Table};

use crate::game::discovery::Discovery;
use crate::messages::action_request::CheatGrantKnowledgeRequest;

#[spacetimedb::reducer]
fn cheat_grant_knowledge(ctx: &ReducerContext, request: CheatGrantKnowledgeRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatGrantKnowledge) {
        return Err("Unauthorized.".into());
    }

    let player_entity_id = request.target_entity_id;
    let also_learn = request.also_learn;

    let mut discovery = Discovery::new(player_entity_id);

    //todo: battle actions

    // buildings
    let building_ids: Vec<i32> = ctx.db.building_desc().iter().map(|x| x.id).collect();
    for building_id in building_ids {
        if also_learn {
            discovery.acquire_building(ctx, building_id);
        } else {
            discovery.discover_building(ctx, building_id);
        }
    }

    // cargos
    let cargo_ids: Vec<i32> = ctx.db.cargo_desc().iter().map(|x| x.id).collect();
    for cargo_id in cargo_ids {
        if also_learn {
            discovery.acquire_cargo(ctx, cargo_id);
        } else {
            discovery.discover_cargo(ctx, cargo_id);
        }
    }

    // construction
    let construction_ids: Vec<i32> = ctx.db.construction_recipe_desc().iter().map(|x| x.id).collect();
    for construction_id in construction_ids {
        if also_learn {
            discovery.acquire_construction(ctx, construction_id);
        } else {
            discovery.discover_construction(ctx, construction_id);
        }
    }

    // craft
    let craft_ids: Vec<i32> = ctx.db.crafting_recipe_desc().iter().map(|x| x.id).collect();
    for craft_id in craft_ids {
        if also_learn {
            discovery.acquire_craft(ctx, craft_id);
        } else {
            discovery.discover_craft(ctx, craft_id);
        }
    }

    // enemy
    let enemy_ids: Vec<i32> = ctx.db.enemy_desc().iter().map(|x| x.enemy_type).collect();
    for enemy_id in enemy_ids {
        if also_learn {
            discovery.acquire_enemy(ctx, enemy_id);
        } else {
            discovery.discover_enemy(ctx, enemy_id);
        }
    }

    // extract
    let extraction_recipe_ids: Vec<i32> = ctx.db.extraction_recipe_desc().iter().map(|x| x.id).collect();
    for extraction_recipe_id in extraction_recipe_ids {
        if also_learn {
            discovery.acquire_extract(ctx, extraction_recipe_id);
        } else {
            discovery.discover_extract(ctx, extraction_recipe_id);
        }
    }

    // item
    let item_ids: Vec<i32> = ctx.db.item_desc().iter().map(|x| x.id).collect();
    for item_id in item_ids {
        if also_learn {
            discovery.acquire_item(ctx, item_id);
        } else {
            discovery.discover_item_and_item_list(ctx, item_id);
        }
    }

    // lore
    let lore_ids: Vec<i32> = ctx.db.knowledge_scroll_desc().iter().map(|x| x.item_id).collect();
    for lore_id in lore_ids {
        if also_learn {
            discovery.acquire_lore(ctx, lore_id);
        } else {
            discovery.discover_lore(ctx, lore_id);
        }
    }

    // npc
    let npc_ids: Vec<i32> = ctx.db.npc_desc().iter().map(|x| x.npc_type).collect();
    for npc_id in npc_ids {
        if also_learn {
            discovery.acquire_npc(ctx, npc_id);
        } else {
            discovery.discover_npc(ctx, npc_id);
        }
    }

    // paving
    let paving_ids: Vec<i32> = ctx.db.paving_tile_desc().iter().map(|x| x.id).collect();
    for paving_id in paving_ids {
        if also_learn {
            discovery.acquire_paving(ctx, paving_id);
        } else {
            discovery.discover_paving(ctx, paving_id);
        }
    }

    // resource
    let resource_ids: Vec<i32> = ctx.db.resource_desc().iter().map(|x| x.id).collect();
    for resource_id in resource_ids {
        if also_learn {
            discovery.acquire_resource(ctx, resource_id);
        } else {
            discovery.discover_resource(ctx, resource_id);
        }
    }

    // resource placement
    let resource_placement_ids: Vec<i32> = ctx.db.resource_placement_recipe_desc().iter().map(|x| x.id).collect();
    for resource_placement_id in resource_placement_ids {
        if also_learn {
            discovery.acquire_resource_placement(ctx, resource_placement_id);
        } else {
            discovery.discover_resource_placement(ctx, resource_placement_id);
        }
    }

    // ruins (skipped)

    // secondary knowledge
    let secondary_ids: Vec<i32> = ctx.db.secondary_knowledge_desc().iter().map(|x| x.id).collect();
    for secondary_id in secondary_ids {
        if also_learn {
            discovery.acquire_secondary(ctx, secondary_id);
        } else {
            discovery.discover_secondary(ctx, secondary_id);
        }
    }

    // vault
    let vault_ids: Vec<i32> = ctx.db.collectible_desc().iter().map(|x| x.id).collect();
    for vault_id in vault_ids {
        if also_learn {
            discovery.acquire_vault(ctx, vault_id);
        } else {
            discovery.discover_vault(ctx, vault_id);
        }
    }

    // deployable
    let deployable_ids: Vec<i32> = ctx.db.deployable_desc().iter().map(|x| x.id).collect();
    for deployable_id in deployable_ids {
        if also_learn {
            discovery.acquire_deployable(ctx, deployable_id);
        } else {
            discovery.discover_deployable(ctx, deployable_id);
        }
    }

    // pillar shaping
    let pillar_shaping_ids: Vec<i32> = ctx.db.pillar_shaping_desc().iter().map(|x| x.id).collect();
    for pillar_id in pillar_shaping_ids {
        if also_learn {
            discovery.acquire_pillar_shaping(ctx, pillar_id);
        } else {
            discovery.discover_pillar_shaping(ctx, pillar_id);
        }
    }

    discovery.commit(ctx);

    log::debug!("Grant knowledge cheat committed.");

    Ok(())
}
