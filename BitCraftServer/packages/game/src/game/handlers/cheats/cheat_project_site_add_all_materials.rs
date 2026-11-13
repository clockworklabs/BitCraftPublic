use crate::{
    construction_recipe_desc,
    game::handlers::cheats::cheat_type::{can_run_cheat, CheatType},
    messages::game_util::{InputItemStack, ItemStack, ItemType},
    project_site_state, resource_placement_recipe_desc, unwrap_or_err,
};
use spacetimedb::ReducerContext;

#[spacetimedb::reducer]
pub fn cheat_project_site_add_all_materials(ctx: &ReducerContext, project_site_entity_id: u64) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatProjectSiteAddAllMaterials) {
        return Err("Unauthorized.".into());
    }

    let mut project_site = unwrap_or_err!(
        ctx.db.project_site_state().entity_id().find(project_site_entity_id),
        "Invalid project site"
    );
    let construction_recipe = ctx.db.construction_recipe_desc().id().find(project_site.construction_recipe_id);
    let resource_placement_recipe = ctx
        .db
        .resource_placement_recipe_desc()
        .id()
        .find(project_site.resource_placement_recipe_id);

    let consumed_cargo_stacks: Vec<InputItemStack>;
    let consumed_item_stacks: Vec<InputItemStack>;
    if let Some(recipe) = construction_recipe {
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        consumed_item_stacks = recipe.consumed_item_stacks;
    } else {
        let recipe = resource_placement_recipe.unwrap();
        consumed_cargo_stacks = recipe.consumed_cargo_stacks;
        consumed_item_stacks = recipe.consumed_item_stacks;
    }

    project_site.cargos.clear();
    for cargo in consumed_cargo_stacks {
        project_site
            .cargos
            .push(ItemStack::new(ctx, cargo.item_id, ItemType::Cargo, cargo.quantity));
    }

    project_site.items.clear();
    for item in consumed_item_stacks {
        project_site
            .items
            .push(ItemStack::new(ctx, item.item_id, ItemType::Item, item.quantity));
    }

    ctx.db.project_site_state().entity_id().update(project_site);

    Ok(())
}
