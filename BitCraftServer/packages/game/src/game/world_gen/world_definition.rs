use crate::messages::world_gen::WorldGenWorldDefinition;

use super::super::unity_helpers::{animation_curve::AnimationCurve, vector2int::Vector2Int};

use super::biome_definition::BiomeDefinition;
use super::building_details::BuildingDetails;
use super::land_shape_definition::LandShapeDefinition;
use super::mountains_map_definition::Mountain;
use super::noise_based_elevation_layer::NoiseBasedElevationLayer;
use super::noise_specs::NoiseSpecs;
use super::resource_definition::{ResourceBiome, ResourceDefinition, ResourceDetails};
use super::river_generation_settings::RiverGenerationSettings;
use super::{
    biomes_map_definition::BiomesMapDefinition, mountains_map_definition::MountainsMapDefinition,
    resources_map_definition::ResourcesMapDefinition, world_map_definition::WorldMapDefinition,
};

pub const TERRAIN_CHUNK_WIDTH: i32 = 32;
pub const TERRAIN_CHUNK_HEIGHT: i32 = 32;
pub const STEP: i32 = 5;

pub struct WorldDefinition {
    pub size: Vector2Int,

    pub land_curve: AnimationCurve,
    pub noise_influence: f32, //[0..1]
    pub sea_level: i16,       //[1..128]

    pub world_map: WorldMapDefinition,

    pub biomes_map: BiomesMapDefinition,
    pub mountains_map: MountainsMapDefinition,
    pub buildings_map: Vec<BuildingDetails>,
    pub resources_map: ResourcesMapDefinition,
}

impl WorldDefinition {
    pub fn new_proto(def: WorldGenWorldDefinition) -> WorldDefinition {
        //WorldMapDefinition
        let world_map = def.world_map;
        let mut shapes: Vec<LandShapeDefinition> = Vec::new();
        for land_shape_definition in world_map.shapes {
            shapes.push(LandShapeDefinition::new(&land_shape_definition));
        }

        let world_map = WorldMapDefinition {
            debug_step: 0,
            shapes,
        };

        //BiomesMapDefinition
        let biomes_map = def.biomes_map;
        let mut biomes: Vec<BiomeDefinition> = Vec::new();
        for biome in biomes_map.biomes {
            let mut n = BiomeDefinition {
                distance_to_sea_curve: biome.distance_to_sea_curve,
                distance_to_biomes_curve: biome.distance_to_biomes_curve,
                transition_length: biome.transition_length,
                noise_sea_multiplier: biome.noise_sea_multiplier,
                noise_based_elevation_layers: Vec::new(),
                max_lake_depth: biome.max_lake_depth as i16,
                terracing: biome.terracing,
                grass_density: biome.grass_density as i16,
                lake_noise_specs: NoiseSpecs::new(&biome.lake_noise_specs, 100000f32),
                lake_noise_threshold: biome.lake_noise_threshold,
                lake_depth_multiplier: biome.lake_depth_multiplier as i16,
                lake_depth_smoothing: biome.lake_depth_smoothing,
                lake_sea_barriers: biome.lake_sea_barriers,
                river_generation_settings: None,
            };

            for noise_based_elevation_layer in biome.noise_based_elevation_layers {
                n.noise_based_elevation_layers
                    .push(NoiseBasedElevationLayer::new(noise_based_elevation_layer));
            }

            if let Some(river_generation_settings) = biome.river_generation_settings {
                n.river_generation_settings = Some(RiverGenerationSettings::new(river_generation_settings));
            }

            biomes.push(n);
        }
        let biomes_map = BiomesMapDefinition {
            biomes,
            values: biomes_map.values,
        };

        //MountainsMapDefinition
        let mut mountains: Vec<Mountain> = Vec::new();
        for m in def.mountains_map.mountains {
            let n = Mountain {
                center: m.center,
                radius: m.radius,
                height: m.height,
                peak_offset: m.peak_offset,
                shape: m.shape,
            };
            mountains.push(n);
        }
        let mountains_map = MountainsMapDefinition { mountains };

        //BuildingsMapDefinition
        let mut buildings_list: Vec<BuildingDetails> = vec![];
        let buildings_map = def.buildings_map;

        for world_gen_building in buildings_map.buildings {
            buildings_list.push(BuildingDetails {
                index: world_gen_building.index,
                id: world_gen_building.id,
                direction: world_gen_building.direction,
            })
        }

        //ResourcesMapDefinition
        let resources_map = def.resources_map;
        let mut resources: Vec<ResourceDefinition> = Vec::new();
        for e in resources_map.resources {
            let resource_details = e.resource_details;
            let resource_details = ResourceDetails {
                clump_id: resource_details.clump_id,
                spawns_on_land: resource_details.spawns_on_land,
                land_elevation_range: resource_details.land_elevation_range,
                spawns_in_water: resource_details.spawns_in_water,
                max_elevation_delta: resource_details.max_elevation_delta,
                water_depth_range: resource_details.water_depth_range,
            };
            let mut bv: Vec<ResourceBiome> = Vec::new();
            for biome in e.biomes {
                let n = ResourceBiome {
                    biome_index: biome.biome_definition_index,
                    chance: biome.chance,
                    noise_threshold: biome.noise_threshold,
                    noise_specs: NoiseSpecs::new(&biome.noise_specs, 100000f32),
                };
                bv.push(n);
            }
            let n = ResourceDefinition {
                resource_details,
                biomes: bv,
            };
            resources.push(n);
        }
        let resources_map = ResourcesMapDefinition {
            seed: resources_map.seed,
            resources,
        };

        //WorldDefinition
        WorldDefinition {
            size: def.size,
            land_curve: def.land_curve,
            noise_influence: def.noise_influence,
            sea_level: def.sea_level as i16,
            world_map,
            biomes_map,
            mountains_map,
            buildings_map: buildings_list,
            resources_map,
        }
    }
}
