use crate::game::game_state;
use crate::game::handlers::cheats::cheat_type::{can_run_cheat, CheatType};

use crate::messages::action_request::CheatCompendiumEnemyPlaceRequest;
use crate::messages::components::{EnemyState, HerdState};
use crate::messages::static_data::EnemyType;
use crate::{enemy_desc, herd_state};

use spacetimedb::ReducerContext;
use spacetimedb::{log, Table};

// Similar to: enemy_spawner_agent.rs
// Getting Herd : enemy_spawner_agent.reduce()
#[spacetimedb::reducer]
pub fn cheat_compendium_place_enemy(ctx: &ReducerContext, request: CheatCompendiumEnemyPlaceRequest) -> Result<(), String> {
    if !can_run_cheat(ctx, &ctx.sender(), CheatType::CheatCompendiumPlaceEnemy) {
        return Err("Unauthorized.".into());
    }

    let enemy_type: EnemyType = request.enemy_type;

    // herd_stub TODO: we might improve this cheat by creating a new herdCache at the specified location with the right biome and EnemyType
    // let enemyState: EnemyState = create_enemy_state(enemy_type, request.coordinates.into(), herd_id);

    let mut herd = HerdState::new(ctx, 0); // DAB Note: it would be nice to be able to set a herd state in interiors
    herd.current_population = 1;
    let herd_entity_id = herd.entity_id;
    game_state::insert_location(ctx, herd_entity_id, request.coordinates);
    let enemy_state = EnemyState::new(ctx, enemy_type, herd_entity_id);
    let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type as i32).unwrap();

    match EnemyState::spawn_enemy(ctx, &enemy_desc, enemy_state, request.coordinates, Some(&herd)) {
        Ok(()) => {}
        Err(s) => log::error!("{}", s),
    };

    log::debug!(
        "[Cheat] cheat_compendium_place_enemy(): Incremented herd.current_population to {}",
        herd.current_population
    );

    let _ = ctx.db.herd_state().try_insert(herd);

    Ok(())
}
