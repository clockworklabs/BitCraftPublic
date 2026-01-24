use spacetimedb::{ReducerContext, Table, log};

use crate::{
    game::{game_state, handlers::authentication::has_role},
    messages::{
        authentication::Role,
        components::{QuestChainState, onboarding_state, quest_chain_state}
    },
};

#[spacetimedb::reducer]
pub fn migrate_onboarding(ctx: &ReducerContext) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender, Role::Admin) {
        return Err("Unauthorized".into());
    }

    let mut count = 0;

    // Maps onboarding state to quest chain desc id.
    // If the player has completed this onboarding state, then they've fully completed the mapped chain.
    let mapping: Vec<(u16, i32)> = vec![
        (48038, 1),             // Onboarding
        (46006, 1511437178),    // Make a cart
        (59472, 751817487),     // Basic Combat tutorial
        (35351, 534319745),     // Town Tour
        (3992, 1708022223),     // Ruined Town Tour
        (58545, 1043927455),    // Achievements + Titles Tut
        (8861, 1918226120),     // Cooking Quest
        (62463, 507534648),     // Ramparte Traveler Task
        (51, 976217825),        // Temple 1
        (19728, 336599736),     // Temple 2
        (15782, 333616920),     // Temple 3
        (47445, 1793465882),    // Temple 4
        (7503, 1699433943),     // Temple 5
        (42, 184422323),        // Cargo hint
        (69, 1370444347),       // Claims hint
        (58, 290593440),        // Cold biome hint
        (28, 1604930937),       // Desert hint
        (79, 679715144),        // KO hint
        (31788, 627872387),     // Starving hint
        (95, 1441436391),       // Vault hint
        (10010, 1358679061)     // Cargo in Deployable hint
    ];

    // 38668 -> 354296535 (Tool quest)

    for onboarding_state in ctx.db.onboarding_state().iter() {
        for (onboarding_id, chain_id) in &mapping {
            if onboarding_state.completed_states.contains(&onboarding_id) {
                complete_chain(ctx, onboarding_state.entity_id, *chain_id)?;
            }
        }

        // Make T1 tool quest, it's the only chain that ends on a quest rather than a state.
        if onboarding_state.completed_quests.contains(&38668) {
            complete_chain(ctx, onboarding_state.entity_id, 354296535)?;
        }
        count += 1;
    }

    log::info!("Migrated {count} players");

    Ok(())
}

fn complete_chain(ctx: &ReducerContext, player_id: u64, chain_id: i32) -> Result<(), String> {
    ctx.db.quest_chain_state().try_insert(QuestChainState{
        entity_id: game_state::create_entity(ctx),
        player_entity_id: player_id,
        quest_chain_desc_id: chain_id,
        stage_id: -1,
        is_active: false,
        completed: true,
        stage_rewards_awarded: Vec::new(),
    })?;
    Ok(())
}