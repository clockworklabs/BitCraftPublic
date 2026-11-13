use spacetimedb::{rand::Rng, ReducerContext};

use crate::{
    loot_table_desc,
    messages::static_data::{ChestRarityDesc, LootTableDesc},
};

impl ChestRarityDesc {
    pub fn rarity_probability(&self, rarity_id: i32) -> f32 {
        if let Some(lr) = self.loot_rarities.iter().find(|clr| clr.rarity == rarity_id) {
            lr.probability
        } else {
            0f32
        }
    }

    pub fn roll(&self, ctx: &ReducerContext, loot_table_indices: &[i32], verbose: bool) -> Option<LootTableDesc> {
        let loot_table_chances: Vec<(f32, LootTableDesc)> = loot_table_indices
            .iter()
            .map(|li| {
                let lt = ctx.db.loot_table_desc().id().find(li).unwrap();
                let prob = self.rarity_probability(lt.loot_rarity);
                if verbose {
                    spacetimedb::log::debug!("-- Loot Table Roll - id: {}, probability: {}", *li, prob);
                }
                (prob, lt)
            })
            .collect();

        let total_weight: f32 = loot_table_chances.iter().map(|l| l.0).sum();
        if total_weight > 0f32 {
            let roll = ctx.rng().gen_range(0.0..total_weight);
            let mut cur_min = 0f32;
            for loot_table_chance in loot_table_chances {
                cur_min += loot_table_chance.0;
                if roll < cur_min {
                    if verbose {
                        spacetimedb::log::debug!("-- Loot Table Roll selected id: {}", loot_table_chance.1.id);
                    }
                    return Some(loot_table_chance.1);
                }
            }
        }

        if verbose {
            spacetimedb::log::debug!("-- Loot Table Roll failed");
        }
        None
    }
}
