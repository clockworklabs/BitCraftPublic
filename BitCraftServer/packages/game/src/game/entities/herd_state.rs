use spacetimedb::rand::Rng;
use spacetimedb::{log, ReducerContext, Table};

use crate::{attached_herds_state, herd_state};
use crate::{game::game_state, messages::util::OffsetCoordinatesSmallMessage, AttachedHerdsState, HerdState};

impl HerdState {
    pub fn new(ctx: &ReducerContext, enemy_ai_params_desc_id: i32) -> HerdState {
        HerdState {
            entity_id: game_state::create_entity(ctx),
            enemy_ai_params_desc_id,
            current_population: 0,
            ignore_eagerness: true,
            population_variance: vec![ctx.rng().gen_range(0.0..1.0), ctx.rng().gen_range(0.0..1.0)],
            crumb_trail_entity_id: 0,
        }
    }

    /*
        let max_population = Self::calculate_max_population(ctx, params.avg_herd_size, params.var_herd_size);
        pub fn calculate_max_population(ctx: &ReducerContext, avg_herd_size: i32, var_herd_size: f32) -> i32 {
        let dist = Gaussian::new(avg_herd_size as f64, var_herd_size as f64);
        let sample = dist.inverse(ctx.rng().gen());
        sample.ceil() as i32
    }
    */

    pub fn attach(ctx: &ReducerContext, host_entity_id: u64, enemy_params_id: Vec<i32>, spawn_location: OffsetCoordinatesSmallMessage) {
        if !enemy_params_id.is_empty() {
            let mut herds_entity_ids = Vec::new();
            for enemy_params_id in enemy_params_id {
                let herd = HerdState::new(ctx, enemy_params_id);
                let herd_entity_id = herd.entity_id;
                herds_entity_ids.push(herd_entity_id);
                if let Err(err) = ctx.db.herd_state().try_insert(herd) {
                    log::error!("{}", err);
                }
                game_state::insert_location(ctx, herd_entity_id, spawn_location);
            }
            if let Err(err) = ctx.db.attached_herds_state().try_insert(AttachedHerdsState {
                entity_id: host_entity_id,
                herds_entity_ids,
            }) {
                log::error!("{}", err);
            }
        }
    }
}

impl AttachedHerdsState {
    pub fn delete(ctx: &ReducerContext, entity_id: u64) {
        if let Some(attached_herds) = ctx.db.attached_herds_state().entity_id().find(entity_id) {
            for herd_entity_id in attached_herds.herds_entity_ids {
                ctx.db.herd_state().entity_id().delete(herd_entity_id);
            }
        }
        ctx.db.attached_herds_state().entity_id().delete(entity_id);
    }
}
