use spacetimedb::{Identity, ReducerContext};

use crate::messages::{
    authentication::{identity_role, Role},
    generic::config,
};

pub enum CheatType {
    CheatUserSetName,
    CheatEmpireSiegeCancel,
    CheatEmpireSiegeAddSupplies,
    CheatShardsGrant,
}

pub fn can_run_cheat(ctx: &ReducerContext, identity: &Identity, cheat_type: CheatType) -> bool {
    if let Some(config) = ctx.db.config().version().find(0) {
        if config.env == "dev" {
            return true; // dev config allows all operations
        }
    }

    let role = match ctx.db.identity_role().identity().find(identity) {
        Some(entry) => entry.role,
        None => return false,
    };

    match cheat_type {
        CheatType::CheatUserSetName => role as i32 >= Role::Mod as i32,

        CheatType::CheatEmpireSiegeCancel => role as i32 >= Role::Gm as i32,
        CheatType::CheatEmpireSiegeAddSupplies => role as i32 >= Role::Gm as i32,
        CheatType::CheatShardsGrant => role as i32 >= Role::Gm as i32,
    }
}
