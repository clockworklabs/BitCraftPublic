
use super::terrain_chunk::TerrainChunkCache;
use crate::game::world_gen::world_definition::{TERRAIN_CHUNK_HEIGHT, TERRAIN_CHUNK_WIDTH};
use crate::game::{coordinates::*, dimensions, game_state};
use crate::messages::components::Biome;
use crate::messages::static_data::BuildingSpawnType;
use crate::{building_desc, building_spawn_desc, building_state, enemy_ai_params_desc, herd_state, location_state, HerdState};
use spacetimedb::rand::Rng;
use spacetimedb::SpacetimeType;
use spacetimedb::{log, ReducerContext, Table};
use strum::IntoEnumIterator;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct RuinsEntityValuePair {
    pub entity_id: u64,
    pub coordinates: SmallHexTile,
}

#[spacetimedb::table(accessor = location_cache)]
pub struct LocationCache {
    #[primary_key]
    pub version: i32,

    pub trading_post_locations: Vec<SmallHexTile>,
    pub all_ruins: Vec<RuinsEntityValuePair>,
    pub traveler_ruins: Vec<RuinsEntityValuePair>,
    pub spawn_locations: Vec<SmallHexTile>,
    // pub enemy_spawn_locations: Vec<Vec<HexCoordinates>>,
    pub biome_chunks: Vec<f32>,
    pub region_max_x: i32,
    pub region_max_z: i32,
    pub region_min_x: i32,
    pub region_min_z: i32,
}

impl LocationCache {
    fn new() -> LocationCache {
        LocationCache {
            version: 0,
            trading_post_locations: Vec::new(),
            all_ruins: Vec::new(),
            traveler_ruins: Vec::new(),
            spawn_locations: Vec::new(),
            biome_chunks: Vec::new(),
            // enemy_spawn_locations: Vec::new(),
            region_max_x: 0,
            region_max_z: 0,
            region_min_x: i32::MAX,
            region_min_z: i32::MAX,
        }
    }

    pub fn get_herd_stubs(&self, ctx: &ReducerContext, biome: Biome, biome_chunk_size: f32) -> Vec<HerdState> {
        let mut herds = Vec::new();
        for param in ctx.db.enemy_ai_params_desc().iter() {
            // only generate new herds or herds that have been cleared by admin reducer
            if ctx.db.herd_state().enemy_ai_params_desc_id().filter(param.id).next().is_some() {
                continue;
            }
            // DAB Note: filter_by_enum isn't supported
            if param.biome == biome {
                let num_herds = f32::ceil(biome_chunk_size * param.herds_per_chunk) as i32;
                for _ in 0..num_herds {
                    herds.push(HerdState::new(ctx, param.id));
                }
            }
        }
        herds
    }

    fn sample_n_locations(&mut self, ctx: &ReducerContext, n: i32) -> Vec<SmallHexTile> {
        let mut locations: Vec<SmallHexTile> = Vec::new();

        let min_x = self.region_min_x * 3;
        let min_z = self.region_min_z * 3;
        let max_x = self.region_max_x * 3;
        let max_z = self.region_max_z * 3;

        for _ in 0..n {
            let coord = OffsetCoordinatesSmall {
                x: ctx.rng().gen_range(min_x..max_x),
                z: ctx.rng().gen_range(min_z..max_z),
                dimension: 1,
            };
            locations.push(SmallHexTile::from(coord));
        }

        locations
    }

    pub fn build_enemy_spawn_locations(&mut self, ctx: &ReducerContext, terrain_cache: &mut TerrainChunkCache) {
        let mut locations: Vec<SmallHexTile> = Vec::new();

        for biome in 0..self.biome_chunks.len() {
            let biome_enum = Biome::to_enum(biome as u8);
            log::info!("Biome {:?} : {} chunks detected.", biome_enum, self.biome_chunks[biome]);

            let mut herd_stubs = self.get_herd_stubs(ctx, biome_enum, self.biome_chunks[biome]);

            while herd_stubs.len() > 0 {
                let samples = self.sample_n_locations(ctx, 200);
                let mut chosen: Option<SmallHexTile> = None;
                let mut max_distance = 0;
                for sample in samples {
                    // skip submerged or inexisting cells
                    match terrain_cache.get_terrain_cell(ctx, &sample.parent_large_tile()) {
                        Some(c) => {
                            if c.is_submerged() || c.biome() != biome as i32 {
                                continue;
                            } else {
                                c
                            }
                        }
                        None => {
                            continue;
                        }
                    };

                    let mut min_distance = i32::MAX;
                    for location in &locations {
                        let distance = sample.distance_to(*location);
                        if distance < min_distance {
                            min_distance = distance;
                        }
                    }
                    if min_distance > max_distance {
                        chosen = Some(sample);
                        max_distance = min_distance;
                    }
                }
                match chosen {
                    Some(coord) => {
                        locations.push(coord);
                        let herd = herd_stubs.remove(0);
                        let herd_entity_id = herd.entity_id;
                        if let Err(err) = ctx.db.herd_state().try_insert(herd) {
                            log::error!("{}", err);
                        }
                        game_state::insert_location(ctx, herd_entity_id, coord.into());
                    }
                    None => {
                        let herd = herd_stubs.remove(0);
                        log::warn!(
                            "Unable to find a suitable location to generate a EnemyAIParams {:?} herd",
                            herd.enemy_ai_params_desc_id
                        );
                    }
                }
            }
        }

        let count: usize = ctx.db.herd_state().iter().count();
        log::info!("Generated {} herd locations.", count);
    }

    fn get_traveler_ruins_ids(&self, ctx: &ReducerContext) -> Vec<i32> {
        // get all ruins with a traveler
        // NOTE: We do not support more than 1 traveler per ruin
        ctx.db
            .building_spawn_desc()
            .iter()
            .filter(|bs| bs.spawn_type == BuildingSpawnType::TravelerCamp)
            .map(|bs| bs.building_id)
            .collect()
    }

    fn collect_traveler_ruins_ids(&mut self, ctx: &ReducerContext) {
        let ruin_ids = self.get_traveler_ruins_ids(ctx);

        self.traveler_ruins = ctx
            .db
            .building_state()
            .iter()
            .filter_map(|b| {
                if ruin_ids.contains(&b.building_description_id) {
                    let coordinates = ctx.db.location_state().entity_id().find(&b.entity_id).unwrap().coordinates();
                    if coordinates.dimension == dimensions::OVERWORLD {
                        return Some(RuinsEntityValuePair {
                            entity_id: b.entity_id,
                            coordinates,
                        });
                    }
                    None
                } else {
                    None
                }
            })
            .collect();

        log::info!("Found {} traveler ruins in world.", self.traveler_ruins.len());
    }

    fn get_all_ruins_ids(&self, ctx: &ReducerContext) -> Vec<i32> {
        let mut ruins: Vec<i32> = ctx
            .db
            .building_desc()
            .iter()
            .filter_map(|b| if b.is_ruins { Some(b.id) } else { None })
            .collect();
        ruins.sort();
        ruins.dedup();
        ruins
    }

    fn collect_all_ruins_ids(&mut self, ctx: &ReducerContext) {
        let ruin_ids = self.get_all_ruins_ids(ctx);

        self.all_ruins = ctx
            .db
            .building_state()
            .iter()
            .filter(|b| ruin_ids.contains(&b.building_description_id))
            .map(|b| RuinsEntityValuePair {
                entity_id: b.entity_id,
                coordinates: ctx.db.location_state().entity_id().find(&b.entity_id).unwrap().coordinates(),
            })
            .collect();

        log::info!("Found a total of {} ruins.", self.all_ruins.len());
    }

    pub fn build(ctx: &ReducerContext) {
        let mut cache = LocationCache::new();
        let mut terrain_cache = TerrainChunkCache::fetch(ctx);

        let biomes_count = Biome::iter().count();
        for _ in 0..biomes_count {
            cache.biome_chunks.push(0.0);
        }

        // spawning points
        for terrain_chunk in terrain_cache.iter_cached_values() {
            for terrain_cell in terrain_chunk.local_iter(1) {
                if terrain_cell.zoning_type == 1 && !terrain_cell.is_submerged() {
                    cache.spawn_locations.push(terrain_cell.coordinates().center_small_tile());
                }
                cache.region_max_x = std::cmp::max(terrain_cell.x, cache.region_max_x);
                cache.region_max_z = std::cmp::max(terrain_cell.z, cache.region_max_z);
                cache.region_min_x = std::cmp::min(terrain_cell.x, cache.region_min_x);
                cache.region_min_z = std::cmp::min(terrain_cell.z, cache.region_min_z);
                cache.biome_chunks[terrain_cell.biome() as usize] += 1.0;
            }
        }

        // divide biome sizes by chunk size
        for chunk_size in cache.biome_chunks.iter_mut() {
            *chunk_size /= (TERRAIN_CHUNK_HEIGHT * TERRAIN_CHUNK_WIDTH) as f32;
        }

        cache.region_max_x += 1;
        cache.region_max_z += 1;

        log::info!("Starting location count {}", cache.spawn_locations.len());

        if cache.region_max_x == 1 && cache.region_max_z == 1 {
            log::info!("No world loaded, not building enemy spawn location cache.");
        } else {
            log::debug!("Started building enemy spawn location cache");
            cache.build_enemy_spawn_locations(ctx, &mut terrain_cache);
            log::debug!("Done building enemy spawn location cache");
        }

        cache.collect_traveler_ruins_ids(ctx);
        cache.collect_all_ruins_ids(ctx);

        if ctx.db.location_cache().try_insert(cache).is_err() {
            log::error!("Failed to insert location cache");
        }
    }
}
