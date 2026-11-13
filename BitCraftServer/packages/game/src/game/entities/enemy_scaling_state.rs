use spacetimedb::ReducerContext;

use crate::messages::{
    components::{enemy_scaling_state, enemy_state, threat_state, EnemyScalingState},
    static_data::{enemy_scaling_desc, EnemyScalingDesc},
};

impl EnemyScalingState {
    pub fn update(ctx: &ReducerContext, enemy_entity_id: u64) {
        if let Some(enemy_state) = ctx.db.enemy_state().entity_id().find(enemy_entity_id) {
            let enemy_type_id = enemy_state.enemy_type as i32;
            let scaling_data: Vec<EnemyScalingDesc> = ctx.db.enemy_scaling_desc().enemy_type_id().filter(enemy_type_id).collect();
            if scaling_data.is_empty() {
                // This enemy is not subject to player scaling
                return;
            }

            // Note: Might have to rethink this if threat can be a non-player for an entity with scaling data
            let threat_count = ctx.db.threat_state().owner_entity_id().filter(enemy_entity_id).count() as i32;

            // Find the highest enemy scaling entry that matches the threat state entries
            let mut highest_scaling: Option<EnemyScalingDesc> = None;
            for data in scaling_data {
                if threat_count >= data.required_players_count
                    && (highest_scaling.is_none() || highest_scaling.as_ref().unwrap().required_players_count < data.required_players_count) {
                        highest_scaling = Some(data);
                    }
            }

            if let Some(scaling) = highest_scaling {
                ctx.db.enemy_scaling_state().entity_id().insert_or_update(EnemyScalingState {
                    entity_id: enemy_entity_id,
                    enemy_scaling_id: scaling.id,
                });
            } else {
                // No more scaling for this enemy
                ctx.db.enemy_scaling_state().entity_id().delete(enemy_entity_id);
            }
        }
    }
}
