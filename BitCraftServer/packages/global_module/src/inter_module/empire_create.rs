use spacetimedb::{ReducerContext, Table};

use crate::messages::components::{previous_empire_name_state, user_state};
use crate::messages::empire_schema::{
    empire_directive_state, empire_emblem_state, empire_log_state, empire_player_log_state, EmpireDirectiveState, EmpireEmblemState,
    EmpireLogState, EmpirePlayerLogState,
};
use crate::messages::static_data::*;
use crate::{
    game::game_state,
    messages::{components::claim_state, empire_shared::*, inter_module::*, static_data::parameters_desc},
    unwrap_or_err,
};

pub fn process_message_on_destination(ctx: &ReducerContext, request: EmpireCreateMsg) -> Result<(), String> {
    let claim = unwrap_or_err!(
        ctx.db.claim_state().owner_building_entity_id().find(request.building_entity_id),
        "This is not a claim"
    );

    if claim.owner_player_entity_id != request.player_entity_id {
        return Err("Not the owner of this claim".into());
    }

    let name_lower = request.empire_name.to_lowercase();
    if ctx.db.empire_lowercase_name_state().name_lowercase().find(&name_lower).is_some() {
        return Err("An empire with this name already exists".into());
    }

    let mut settlement = unwrap_or_err!(
        ctx.db
            .empire_settlement_state()
            .building_entity_id()
            .find(request.building_entity_id),
        "This claim does not have the tech to form an empire"
    );

    if ctx
        .db
        .empire_icon_desc()
        .id()
        .find(request.icon_id)
        .is_none_or(|icon| icon.is_shape)
    {
        return Err("Invalid empire icon".into());
    }

    if ctx
        .db
        .empire_icon_desc()
        .id()
        .find(request.shape_id)
        .is_none_or(|icon| !icon.is_shape)
    {
        return Err("Invalid empire shape".into());
    }

    if ctx.db.empire_color_desc().id().find(request.color1_id).is_none()
        || ctx.db.empire_color_desc().id().find(request.color2_id).is_none()
    {
        return Err("Invalid empire colors".into());
    }

    // Prevent the player from using an empire name assigned to a different player
    let identity = ctx.db.user_state().entity_id().find(request.player_entity_id).unwrap().identity;

    if let Some(previous_entry) = ctx
        .db
        .previous_empire_name_state()
        .empire_lower_case_name()
        .find(request.empire_name.to_lowercase())
    {
        if previous_entry.emperor_identity != identity {
            return Err("This name is unavailable".into());
        }
    }

    // Clear player's reserved name, if any, whether it used the previous name or not
    ctx.db.previous_empire_name_state().emperor_identity().delete(identity);

    let params = ctx.db.parameters_desc().version().find(0).unwrap();

    let empire_entity_id = game_state::create_entity(ctx);

    let empire_default_nobility_threshold = params.empire_default_nobility_threshold;

    // Create Empire
    let empire = EmpireState {
        entity_id: empire_entity_id,
        capital_building_entity_id: request.building_entity_id,
        name: request.empire_name,
        shard_treasury: params.empire_starting_shards as u32,
        empire_currency_treasury: params.empire_starting_currency,
        nobility_threshold: empire_default_nobility_threshold,
        num_claims: 1,
        location: settlement.location,
        owner_type: EmpireOwnerType::Player,
    };
    EmpireState::insert_shared(ctx, empire, crate::inter_module::InterModuleDestination::AllOtherRegions);
    ctx.db.empire_lowercase_name_state().insert(EmpireLowercaseNameState {
        entity_id: empire_entity_id,
        name_lowercase: name_lower,
    });
    ctx.db.empire_log_state().try_insert(EmpireLogState {
        entity_id: empire_entity_id,
        last_posted: 0,
    })?;
    ctx.db.empire_emblem_state().insert(EmpireEmblemState {
        entity_id: empire_entity_id,
        icon_id: request.icon_id,
        shape_id: request.shape_id,
        color1_id: request.color1_id,
        color2_id: request.color2_id,
    });
    ctx.db.empire_directive_state().insert(EmpireDirectiveState {
        entity_id: empire_entity_id,
        directive_message: String::new(),
        directive_message_timestamp: None,
    });

    EmpirePlayerDataState::new(ctx, request.player_entity_id, empire_entity_id, 0 /*Emperor*/)?;

    ctx.db.empire_player_log_state().try_insert(EmpirePlayerLogState {
        entity_id: request.player_entity_id,
        empire_entity_id,
        last_viewed: 0,
    })?;

    // Create default ranks for the new empire
    for rank_desc in ctx.db.empire_rank_desc().iter() {
        let rank_entity_id = game_state::create_entity(ctx);
        let title = rank_desc.title;
        let permissions = rank_desc.permissions;

        EmpireRankState::insert_shared(
            ctx,
            EmpireRankState {
                entity_id: rank_entity_id,
                empire_entity_id,
                rank: rank_desc.rank as u8,
                title,
                permissions,
            },
            crate::inter_module::InterModuleDestination::AllOtherRegions,
        );
    }

    settlement.empire_entity_id = empire_entity_id;
    EmpireSettlementState::update_shared(ctx, settlement, crate::inter_module::InterModuleDestination::AllOtherRegions);

    EmpireState::update_empire_upkeep(ctx, empire_entity_id);

    EmpireState::update_crown_status(ctx, empire_entity_id);

    Ok(())
}
