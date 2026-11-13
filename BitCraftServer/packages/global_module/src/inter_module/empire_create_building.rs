use spacetimedb::{ReducerContext, Table, Timestamp};

use crate::{
    game::{coordinates::SmallHexTile, handlers::empires::empires_shared::validate_empire_build_foundry},
    messages::{
        empire_schema::*,
        empire_shared::*,
        inter_module::EmpireCreateBuildingMsg,
        static_data::{building_desc, construction_recipe_desc, BuildingCategory},
    },
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireCreateBuildingMsg) -> Result<(), String> {
    let building_desc = unwrap_or_err!(ctx.db.building_desc().id().find(request.building_desc_id), "Invalid building");
    let coord = SmallHexTile::from(request.location);

    if building_desc.has_category(ctx, BuildingCategory::EmpireFoundry) {
        validate_empire_build_foundry(ctx, request.player_entity_id, coord)?;
        let empire_entity_id = unwrap_or_err!(
            EmpirePlayerDataState::get_player_empire_id(ctx, request.player_entity_id),
            "Player is not a part of any empire"
        );
        let foundry = EmpireFoundryState {
            entity_id: request.building_entity_id,
            hexite_capsules: 0,
            queued: 0,
            started: Timestamp::UNIX_EPOCH,
            empire_entity_id,
        };
        ctx.db.empire_foundry_state().insert(foundry);
    }

    //Consume shards from empire treasury
    if let Some(construction_recipe_id) = request.construction_recipe_id.filter(|_| request.player_entity_id > 0) {
        let construction_recipe = ctx.db.construction_recipe_desc().id().find(construction_recipe_id).unwrap();
        if construction_recipe.consumed_shards > 0 {
            let player_data = ctx
                .db
                .empire_player_data_state()
                .entity_id()
                .find(request.player_entity_id)
                .unwrap();
            let mut empire = ctx.db.empire_state().entity_id().find(player_data.empire_entity_id).unwrap();
            if empire.empire_currency_treasury < construction_recipe.consumed_shards as u32 {
                return Err("Your empire does not have enough hexite energy".into());
            }
            empire.empire_currency_treasury -= construction_recipe.consumed_shards as u32;
            EmpireState::update_shared(ctx, empire, super::InterModuleDestination::AllOtherRegions);
        }
    }

    Ok(())
}
