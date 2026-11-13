use spacetimedb::ReducerContext;

pub use crate::game::coordinates::*;
use crate::messages::static_data::*;

impl ResourceClumpDesc {
    pub fn footprints(&self, ctx: &ReducerContext) -> Vec<FootprintTile> {
        let mut footprints = Vec::new();
        for i in 0..self.resource_id.len() {
            let resource_id = self.resource_id[i];
            let x = self.x[i];
            let z = self.z[i];
            let resource_desc = ctx.db.resource_desc().id().find(resource_id).unwrap();
            if resource_desc.footprint.is_empty() {
                // default: (0,0)
                footprints.push(FootprintTile {
                    x,
                    z,
                    footprint_type: FootprintType::Hitbox,
                }) // what should be the type? is it important for world generation?
            } else {
                for delta in &resource_desc.footprint {
                    let mut d = delta.clone();
                    d.x += x;
                    d.z += z;
                    footprints.push(d);
                }
            }
        }
        footprints
    }

    pub fn spawn_priority(&self, ctx: &ReducerContext) -> i32 {
        let mut priority = 0;
        for resource_id in self.resource_id.iter() {
            let resource_desc = ctx.db.resource_desc().id().find(resource_id).unwrap();
            priority = priority.max(resource_desc.spawn_priority);
        }
        priority
    }

    pub fn get_resource_ids(ctx: &ReducerContext, resource_clump_id: i32) -> Vec<i32> {
        let resource_clump = ctx.db.resource_clump_desc().id().find(resource_clump_id).unwrap();
        resource_clump.resource_id
    }
}
