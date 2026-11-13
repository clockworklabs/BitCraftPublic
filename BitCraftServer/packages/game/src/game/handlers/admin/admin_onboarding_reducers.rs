use spacetimedb::ReducerContext;

use crate::{game::handlers::authentication::has_role, messages::authentication::Role, onboarding_state, OnboardingState};

#[spacetimedb::reducer]
pub fn admin_reset_onboarding_completely(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![],
        current_quests: vec![],
        completed_quests: vec![],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

// !!THE REDUCERS FROM HERE DOWN HAVE BEEN MADE FOR ALPHA 3, AS WE CHANGE ONBOARDING THESE MIGHT NO LONGER BE VALID!!
#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_first_temple_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232,
        ],
        current_quests: vec![],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_first_expand_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51,
        ],
        current_quests: vec![23],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_second_temple_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72,
        ],
        current_quests: vec![800],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_second_expand_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728,
        ],
        current_quests: vec![58573],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_third_temple_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084,
        ],
        current_quests: vec![43014],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_third_expand_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084,
        ],
        current_quests: vec![28891],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_fourth_temple_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084,
        ],
        current_quests: vec![49022],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_fourth_expand_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084,
        ],
        current_quests: vec![29454],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_reset_onboarding_to_fifth_temple_quest(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084,
        ],
        current_quests: vec![44679],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}

#[spacetimedb::reducer]
pub fn admin_alpha3_complete_onboarding(ctx: &ReducerContext, entity_id: u64) -> Result<(), String> {
    if !has_role(ctx, &ctx.sender(), Role::Admin) {
        return Err("Invalid permissions".into());
    }

    let new_state = OnboardingState {
        entity_id,
        completed_states: vec![
            75, 52, 68, 81, 13, 53, 2, 74, 33, 96, 48038, 61, 54414, 6, 61693, 38, 88, 32, 51732, 22888, 7219, 4623, 46006, 44068, 12368,
            28135, 62232, 27, 84, 11, 0, 18, 63, 51, 72, 26712, 18883, 19728, 52918, 33460, 16084, 7503,
        ],
        current_quests: vec![],
        completed_quests: vec![
            45, 1, 50, 67, 78, 53918, 91, 55, 31, 70, 48930, 49, 40227, 82, 94, 5, 20, 20672, 54970, 34062, 2876, 7363, 4, 62, 15, 59822,
            17, 35, 23, 800, 25, 301, 58573, 11234, 16296,
        ],
    };

    ctx.db.onboarding_state().entity_id().update(new_state);

    Ok(())
}
