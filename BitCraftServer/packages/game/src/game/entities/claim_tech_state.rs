
use spacetimedb::ReducerContext;

use crate::{claim_tech_desc, messages::components::ClaimTechState};

impl ClaimTechState {
    pub fn max_supplies(&self, ctx: &ReducerContext) -> f32 {
        self.learned
            .iter()
            .map(|id| ctx.db.claim_tech_desc().id().find(id).unwrap().supplies)
            .max()
            .unwrap() as f32
    }

    pub fn max_members(&self, ctx: &ReducerContext) -> i32 {
        self.learned
            .iter()
            .map(|id| ctx.db.claim_tech_desc().id().find(id).unwrap().members)
            .max()
            .unwrap()
    }

    pub fn max_tiles(&self, ctx: &ReducerContext) -> i32 {
        self.learned
            .iter()
            .map(|id| ctx.db.claim_tech_desc().id().find(id).unwrap().area)
            .max()
            .unwrap()
    }

    pub fn max_tier(&self, ctx: &ReducerContext) -> i32 {
        self.learned
            .iter()
            .filter_map(|id| ctx.db.claim_tech_desc().id().find(id).map(|desc| desc.tier))
            .max()
            .unwrap_or(0)
    }

    pub fn min_xp_to_mint_hex_coin(&self, ctx: &ReducerContext) -> u32 {
        return self
            .learned
            .iter()
            .filter_map(|id| {
                if let Some(claim_tech_desc) = ctx.db.claim_tech_desc().id().find(id) {
                    if claim_tech_desc.xp_to_mint_hex_coin > 0 {
                        return Some(claim_tech_desc.xp_to_mint_hex_coin);
                    }
                }
                return Some(u32::MAX);
            })
            .min()
            .unwrap();
    }

    pub fn has_unlocked_tech(&self, tech_id: i32) -> bool {
        self.learned.contains(&tech_id)
    }
}
