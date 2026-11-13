use spacetimedb::{ReducerContext, Table};

use crate::game::coordinates::*;
use crate::messages::static_data::*;

impl BuildingDesc {
    pub fn fulfills_function(&self, building_type: i32, level: i32) -> bool {
        self.functions.iter().any(|f| f.function_type == building_type && f.level >= level)
    }

    pub fn get_function(&self, ctx: &ReducerContext, category: BuildingCategory) -> Option<&BuildingFunction> {
        self.functions.iter().find(|f| {
            let building_type = match ctx.db.building_type_desc().id().find(f.function_type) {
                Some(t) => t,
                None => return false,
            };

            building_type.category == category
        })
    }

    pub fn claim_totems(ctx: &ReducerContext) -> Vec<i32> {
        ctx.db
            .building_desc()
            .iter()
            .filter_map(|bd| {
                if bd.has_category(ctx, BuildingCategory::ClaimTotem) {
                    return Some(bd.id);
                }
                None
            })
            .collect()
    }

    pub fn has_category(&self, ctx: &ReducerContext, category: BuildingCategory) -> bool {
        self.get_function(ctx, category).is_some()
    }

    pub fn get_footprint(&self, coordinates: &SmallHexTile, direction: i32) -> Vec<(SmallHexTile, FootprintType)> {
        self.footprint
            .iter()
            .map(|delta| {
                (
                    SmallHexTile {
                        x: coordinates.x + delta.x,
                        z: coordinates.z + delta.z,
                        dimension: coordinates.dimension,
                    }
                    .rotate_around(coordinates, direction / 2),
                    delta.footprint_type,
                )
            })
            .collect()
    }

    pub fn is_enterable(&self) -> bool {
        self.footprint.iter().any(|f| f.footprint_type == FootprintType::Walkable)
    }
}
