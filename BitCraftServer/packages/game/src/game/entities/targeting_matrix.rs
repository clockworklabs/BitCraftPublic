use spacetimedb::ReducerContext;

use crate::{enemy_desc, enemy_state, targeting_matrix_desc, TargetingMatrixDesc};

impl TargetingMatrixDesc {
    pub fn from_enemy_entity_id(ctx: &ReducerContext, enemy_entity_id: u64) -> Option<TargetingMatrixDesc> {
        if let Some(enemy) = ctx.db.enemy_state().entity_id().find(enemy_entity_id) {
            let enemy_type = enemy.enemy_type as i32;
            let enemy_desc = ctx.db.enemy_desc().enemy_type().find(enemy_type).unwrap();
            return Some(ctx.db.targeting_matrix_desc().id().find(enemy_desc.targeting_matrix_id).unwrap());
        }
        None
    }

    pub fn player(ctx: &ReducerContext) -> TargetingMatrixDesc {
        ctx.db.targeting_matrix_desc().id().find(1).unwrap()
    }

    pub fn building(ctx: &ReducerContext) -> TargetingMatrixDesc {
        ctx.db.targeting_matrix_desc().id().find(4).unwrap()
    }

    pub fn can_attack(&self, other: &TargetingMatrixDesc) -> bool {
        self.categories_attacked.contains(&other.id)
    }
}
