use std::collections::{HashMap, HashSet};

use utils::iter_utils::GroupBy;

use crate::*;

pub struct LocationStateCache {
    cached_chunks: HashSet<u64>,
    entities_by_coord: HashMap<i64, Vec<u64>>,
    empty_vec: Vec<u64>,
}

impl Default for LocationStateCache {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationStateCache {
    pub fn new() -> Self {
        Self {
            cached_chunks: HashSet::new(),
            entities_by_coord: HashMap::with_capacity(1024),
            empty_vec: vec![0],
        }
    }

    pub fn select_all(&mut self, ctx: &ReducerContext, coord: &SmallHexTile) -> &Vec<u64> {
        let chunk = coord.chunk_coordinates();
        let chunk_index = chunk.chunk_index();
        if !self.cached_chunks.contains(&chunk_index) {
            self.cache_chunk_impl(ctx, chunk_index);
        }

        let r = self.entities_by_coord.get(&coord.hashcode());
        match r {
            Some(v) => v,
            None => &self.empty_vec,
        }
    }

    pub fn add_location_entity(&mut self, entity_id: u64, coord: &SmallHexTile) {
        let hash = coord.to_offset_coordinates().hashcode();
        self.entities_by_coord
            .entry(hash)
            .or_insert_with(|| Vec::with_capacity(1))
            .push(entity_id);
    }

    fn cache_chunk_impl(&mut self, ctx: &ReducerContext, chunk_index: u64) {
        self.cached_chunks.insert(chunk_index);
        self.entities_by_coord.extend(
            ctx.db
                .location_state()
                .chunk_index()
                .filter(chunk_index)
                .map(|l| (l.offset_coordinates().hashcode(), l.entity_id))
                .group_by(|l| l.0, |l| l.1),
        );
    }
}
