use std::collections::{HashMap, HashSet};
use std::num::Wrapping;

use crate::game::coordinates::hex_coordinates::HexCoordinates;
use crate::game::coordinates::*;
use crate::game::generic::pathfinder::{Edge, Pathfinder};
use crate::game::unity_helpers::common_rng::CommonRNG;
use crate::game::unity_helpers::float_helper::f32::{half_to_even, inverse_lerp, lerp, map};
use crate::game::world_gen::biomes_map_definition::BiomesMapDefinition;
use crate::game::world_gen::resource_definition::{ResourceBiome, ResourceDetails};
use crate::game::world_gen::world_generation::lake::Lake;
use crate::game::world_gen::world_generation::river::River;
use crate::game::world_gen::world_generation::terrain_node::NodeType;
use crate::messages::static_data::*;

use super::super::world_definition;
use super::super::world_definition::WorldDefinition;
use super::entity_node::{EntityNode, ResourceData};
use crate::game::unity_helpers::vector2::Vector2;

use super::hex_graph::HexGraph;
use super::terrain_node::{self, TerrainNode};

pub struct WorldGraph {
    pub terrain_graph: HexGraph<TerrainNode>,
    pub entities_graph: HexGraph<EntityNode>,
}

use spacetimedb::{log, ReducerContext};

impl WorldGraph {
    pub fn new(ctx: &ReducerContext, world_definition: &mut WorldDefinition) -> WorldGraph {
        let terrain_width = world_definition::TERRAIN_CHUNK_WIDTH * world_definition.size.x;
        let terrain_depth = world_definition::TERRAIN_CHUNK_HEIGHT * world_definition.size.y;

        for shape in &mut world_definition.world_map.shapes {
            shape.compute();
        }

        let mut terrain_graph: HexGraph<TerrainNode> = HexGraph::new(terrain_width as usize, terrain_depth as usize);
        let mut entities_graph: HexGraph<EntityNode> = HexGraph::new((terrain_width * 3) as usize, (terrain_depth * 3) as usize);

        log::info!("[1/3] Generating terrain...");
        compute_terrain_graph(world_definition, &mut terrain_graph);
        log::info!("[2/3] Generating Buildings...");
        compute_buildings(world_definition, &mut entities_graph);
        log::info!("[3/3] Generating resources...");
        compute_resources(ctx, world_definition, &mut entities_graph, &terrain_graph);

        // UNCOMMENT FOR RESOURCES LOG
        // let mut indices: HashMap<i32, usize> = HashMap::new();
        // let resources_len = (&world_definition.resources_map).count() as usize;
        // for i in 0..resources_len {
        //     indices.insert(
        //         (&world_definition.resources_map)
        //             .get_resource(i as i32)
        //             .unwrap()
        //             .resource_details
        //             .id,
        //         i,
        //     );
        // }
        // let biomes_len = (&world_definition.biomes_map).count() as usize;
        // let mut total_hex_count = 0f32;
        // let mut count: Vec<i32> = vec![0; resources_len];
        // // let mut biomes_total: Vec<f32> = vec![0f32; biomes_len];
        // // let mut biomes_count: Vec<Vec<i32>> = vec![vec![0; biomes_len]; resources_len];
        // for node in &entities_graph.nodes {
        //     if node.is_underwater(&terrain_graph) || node.building.is_some() {
        //         continue;
        //     }

        //     total_hex_count += 1f32;

        //     let resource_index = if node.resource.is_some() {
        //         Some(indices[&node.resource.unwrap().id])
        //     } else {
        //         None
        //     };

        //     if resource_index.is_some() {
        //         count[resource_index.unwrap()] += 1;
        //     }

        //     // for biome_index in 0..biomes_len {
        //     //     let biome_value = node.get_biome_value(biome_index, &terrain_graph);
        //     //     if biome_value <= 0f32 {
        //     //         continue;
        //     //     }
        //     //     biomes_total[biome_index] += biome_value;
        //     //     if resource_index.is_some() {
        //     //         biomes_count[resource_index.unwrap()][biome_index] += 1;
        //     //     }
        //     // }
        // }

        // log::info!(
        //     "World Area:{0} of small hex in world ≈ {1} of chunks",
        //     total_hex_count,
        //     total_hex_count / 9216f32
        // );

        // for i in 0..resources_len {
        //     log::info!(
        //         "{0} -> Total Count: {1} ≈ {2} per chunk",
        //         (&world_definition.resources_map)
        //             .get_resource(i as i32)
        //             .unwrap()
        //             .resource_details
        //             .id,
        //         count[i],
        //         (count[i] as f32 / total_hex_count) * 9216f32
        //     );
        // }

        WorldGraph {
            terrain_graph,
            entities_graph,
        }
    }
}

fn compute_terrain_graph(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
    compute_terrain_types(world_definition, graph);
    compute_terrain_distances(world_definition, graph);
    set_elevation(world_definition, graph);
    compute_noise_based_lakes(world_definition, graph);
    compute_rivers(world_definition, graph);
    set_grass_density(world_definition, graph);

    fn compute_terrain_types(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        let world_map = &world_definition.world_map;

        for i in 0..graph.count() {
            let node = graph.get_mut(i).unwrap();
            let position = node.world_position();
            node.node_type = if world_map.is_water(position) {
                terrain_node::NodeType::Lake
            } else {
                terrain_node::NodeType::Land
            };
        }

        graph.flood_fill(
            0,
            |node| node.node_type == terrain_node::NodeType::Lake,
            |node| node.node_type = terrain_node::NodeType::Sea,
        );
    }

    fn compute_terrain_distances(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        distance_to_sea(graph);
        distance_to_water(graph);
        distances_to_biomes(world_definition, graph);

        fn distance_to_sea(graph: &mut HexGraph<TerrainNode>) {
            graph.distance_to(
                |node| node.node_type == terrain_node::NodeType::Sea,
                |node| node.distance_to_sea,
                |node, value| {
                    node.distance_to_sea = value;
                },
            );

            let mut max = i32::MIN;
            for i in 0..graph.count() {
                let node = graph.get(i).unwrap();
                if node.distance_to_sea > max {
                    max = node.distance_to_sea;
                }
            }

            let i_max = 1f32 / max as f32;

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                node.distance_to_sea_relative = (node.distance_to_sea as f32 * i_max).clamp(0f32, 1f32);
            }
        }
        fn distance_to_water(graph: &mut HexGraph<TerrainNode>) {
            graph.distance_to(
                |node| node.is_water(),
                |node| node.distance_to_water,
                |node, value| {
                    node.distance_to_water = value;
                },
            );

            let mut max = i32::MIN;
            for i in 0..graph.count() {
                let node = graph.get(i).unwrap();
                if node.distance_to_water > max {
                    max = node.distance_to_water;
                }
            }

            let i_max = 1f32 / max as f32;

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                node.distance_to_water_relative = (node.distance_to_water as f32 * i_max).clamp(0f32, 1f32);
            }
        }
        fn distances_to_biomes(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
            let biomes_map = &world_definition.biomes_map;
            let biomes_count = biomes_map.count();

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                node.distances_to_biomes = vec![0; biomes_count as usize];
                node.biomes_multipliers = vec![0f32; biomes_count as usize];
                node.biomes = biomes_map.get_index_at_pos(node.world_position()) as u32;
            }

            for i in 0..biomes_count {
                let index = i;
                graph.distance_to(
                    |node| node.biome() == index as u32,
                    |node| node.distances_to_biomes[index as usize],
                    |node, value| node.distances_to_biomes[index as usize] = value,
                );
            }

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                let mut min = node.distance_to_sea;
                for j in 0..biomes_count {
                    let biome = biomes_map.get_biome(j).unwrap();
                    let distance = node.distances_to_biomes[j as usize];

                    node.biomes_multipliers[j as usize] = if distance < 0 {
                        -1f32
                    } else {
                        (1f32 - distance as f32 / biome.transition_length as f32).clamp(0f32, 1f32)
                    };

                    if node.biome() != j as u32 && distance >= 0 && distance < min {
                        min = distance;
                    }
                }

                node.distance_to_different_biomes = min;
            }

            for i in 0..biomes_count {
                let mut max = i32::MIN;
                for j in 0..graph.count() {
                    let node = graph.get(j).unwrap();
                    if node.biome() != i as u32 {
                        continue;
                    }
                    if node.distance_to_different_biomes > max {
                        max = node.distance_to_different_biomes;
                    }
                }

                let i_max = 1f32 / max as f32;

                for j in 0..graph.count() {
                    let node = graph.get_mut(j).unwrap();
                    if node.biome() != i as u32 {
                        continue;
                    }
                    node.distance_to_different_biomes_relative = (node.distance_to_different_biomes as f32 * i_max).clamp(0f32, 1f32);
                }
            }
        }
    }

    fn set_elevation(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        calculate_terrain_elevation(world_definition, graph);
        calculate_water_level(world_definition, graph);
        extend_water_level(graph);
        calculate_water_elevation(world_definition, graph);

        fn calculate_terrain_elevation(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
            let world_map = &world_definition.world_map;
            let biomes_map = &world_definition.biomes_map;
            let mountains_map = &world_definition.mountains_map;

            let terrace = |elevation: f32| -> f32 {
                let elevation_above_sea_level = elevation - world_definition.sea_level as f32;
                let round_factor = if elevation_above_sea_level < 16f32 {
                    4f32
                } else {
                    if elevation_above_sea_level < 32f32 {
                        8f32
                    } else {
                        16f32
                    }
                };
                let downscaled = elevation / round_factor;
                let deci = downscaled - downscaled.floor();

                if deci < 0.6f32 && deci > 0.4f32 {
                    return downscaled * round_factor;
                } else {
                    return half_to_even(downscaled) * round_factor;
                }
            };

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                let position = node.world_position();
                let noise = world_map.get_noise(position);

                if node.is_water() {
                    continue;
                }

                let land_curve_t = lerp(node.distance_to_sea_relative, noise, world_definition.noise_influence);
                let from_land_curve = world_definition.land_curve.evaluate(land_curve_t).max(0f32);

                let offset = node.coordinates.to_offset_coordinates();

                let mut from_peaks = 0f32;
                let mut noise_multiplier = 1f32;
                for k in 0..mountains_map.count() {
                    let mountain = mountains_map.get(k);

                    let radius = mountain.radius;
                    let sqr_radius = radius * radius;

                    let to_center = position - mountain.center;
                    if to_center.sqr_magnitude() >= sqr_radius {
                        continue;
                    }

                    let peak_position = mountain.center + mountain.peak_offset;
                    let outer_edge = point_on_circumference(peak_position, position, mountain.center, radius);

                    let t = (peak_position - position).magnitude() / (peak_position - outer_edge).magnitude();

                    let mountain_curve_t = lerp(
                        1f32 - t,
                        noise,
                        world_definition.noise_influence * (t * std::f32::consts::PI).sin().powf(0.3f32),
                    );

                    from_peaks = from_peaks.max(mountain.shape.evaluate(mountain_curve_t) * mountain.height as f32);
                    noise_multiplier = noise_multiplier.min(t.powf(0.3f32));
                }

                let mut s = 0f32;
                let mut from_biome = 0f32;
                let mut biome_noise = 0f32;
                for j in 0..node.biomes_multipliers.len() {
                    let biome = biomes_map.get_biome(j as i32);
                    if biome.is_none() {
                        continue;
                    }

                    let biome = biome.unwrap();
                    let multiplier = node.biomes_multipliers[j];
                    if multiplier <= 0f32 {
                        continue;
                    }

                    let distance_to_sea_curve_t = lerp(node.distance_to_sea_relative, noise, world_definition.noise_influence);

                    let distance_to_sea_curve =
                        half_to_even(biome.distance_to_sea_curve.evaluate(distance_to_sea_curve_t).max(0f32)) as i32;

                    let distance_to_biome_curve_t = lerp(
                        if node.biome() == j as u32 {
                            node.distance_to_different_biomes_relative
                        } else {
                            0f32
                        },
                        noise,
                        world_definition.noise_influence,
                    );

                    let distance_to_biome_curve =
                        half_to_even(biome.distance_to_biomes_curve.evaluate(distance_to_biome_curve_t).max(0f32)) as i32;

                    let mut from_biome_noise = biome.get_elevation_from_noise_layers(offset.x as f32, offset.z as f32);

                    from_biome_noise *= biome.noise_sea_multiplier.evaluate(node.distance_to_water_relative);

                    s += multiplier;
                    from_biome += (distance_to_sea_curve as f32 + distance_to_biome_curve as f32) * multiplier;
                    biome_noise += from_biome_noise as f32 * multiplier;
                }
                from_biome /= s;
                biome_noise /= s;

                let mut elevation = (from_land_curve + from_biome).max(from_peaks) + biome_noise * noise_multiplier;

                if elevation < 0f32 {
                    elevation *= node.distance_to_sea_relative.powf(0.5f32);
                }

                if biomes_map.get_biome(node.biome() as i32).unwrap().terracing {
                    elevation = terrace(elevation);
                }

                node.elevation = (world_definition.sea_level as f32 + elevation).clamp(0f32, i16::MAX as f32) as i16;
            }
        }

        fn calculate_water_level(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
            for i in 0..graph.count() {
                let node_type: terrain_node::NodeType;
                {
                    let node = graph.get(i).unwrap();
                    node_type = node.node_type.clone();
                }

                match node_type {
                    terrain_node::NodeType::Sea => {
                        let node = graph.get_mut(i).unwrap();
                        node.water_level = world_definition.sea_level;
                    }

                    terrain_node::NodeType::Lake => {
                        let mut min = i16::MAX;
                        for j in 0..6 {
                            let neighbor_index = graph.get_neighbor(i, j);
                            if neighbor_index < 0 {
                                continue;
                            }

                            let neighbor = graph.get(neighbor_index).unwrap();
                            if neighbor.node_type != terrain_node::NodeType::Land {
                                continue;
                            }

                            if neighbor.elevation < min {
                                min = neighbor.elevation;
                            }
                        }

                        let node = graph.get_mut(i).unwrap();
                        node.water_level = min;
                    }

                    _ => {}
                }
            }

            graph.min_flood_fill_all_areas(
                |node| node.node_type == terrain_node::NodeType::Lake,
                |node| node.water_level as i32,
                |node, value| node.water_level = value as i16,
            );
        }

        fn extend_water_level(graph: &mut HexGraph<TerrainNode>) {
            for i in 0..graph.count() {
                let is_water: bool;
                let water_level: i16;
                {
                    let node = graph.get(i).unwrap();
                    is_water = node.is_water();
                    water_level = node.water_level;
                }

                if is_water {
                    continue;
                }

                for j in 0..6 {
                    let neighbor_index = graph.get_neighbor(i, j);
                    if neighbor_index < 0 {
                        continue;
                    }

                    let neighbor_water_level: i16;
                    {
                        let neighbor = graph.get(neighbor_index).unwrap();
                        if !neighbor.is_water() {
                            continue;
                        }
                        neighbor_water_level = neighbor.water_level;
                    }

                    if neighbor_water_level > water_level {
                        let node = graph.get_mut(i).unwrap();
                        node.water_level = neighbor_water_level;
                    }
                }
            }
        }

        fn calculate_water_elevation(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
            let world_map = &world_definition.world_map;
            let biomes_map = &world_definition.biomes_map;

            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();

                if !node.is_water() {
                    continue;
                }

                let position = node.world_position();
                let noise = world_map.get_noise(position);

                if node.node_type == terrain_node::NodeType::Sea {
                    node.elevation = lerp(0f32, world_definition.sea_level as f32, noise) as i16;
                    continue;
                }

                let mut s = 0f32;
                let mut max_lake_depth = 0f32;
                for j in 0..node.biomes_multipliers.len() {
                    let biome = biomes_map.get_biome(j as i32);
                    if biome.is_none() {
                        continue;
                    }
                    let biome = biome.unwrap();
                    let multiplier = node.biomes_multipliers[j];
                    if multiplier < 0f32 {
                        continue;
                    }

                    s += multiplier;
                    max_lake_depth += biome.max_lake_depth as f32 * multiplier;
                }
                max_lake_depth /= s;

                node.elevation = (node.water_level as f32 + lerp(-max_lake_depth, 0f32, noise.powf(3f32))) as i16;
            }
        }
    }

    fn compute_noise_based_lakes(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        let biomes_map = &world_definition.biomes_map;
        compute_depth(biomes_map, graph);
        flatten_floor(graph);
        apply_depth_and_set_water_level(biomes_map, graph);

        // Traverse the graph, sample the lake noise and calculate the depth if the noise is above the threshold
        fn compute_depth(biomes_map: &BiomesMapDefinition, graph: &mut HexGraph<TerrainNode>) {
            for i in 0..graph.count() {
                let node = graph.get_mut(i).unwrap();
                if node.node_type != terrain_node::NodeType::Land {
                    continue;
                }

                let mut multiplier_total = 0f32;
                let mut lake_depth_total = 0f32;

                for j in 0..node.biomes_multipliers.len() {
                    let multiplier = node.biomes_multipliers[j];
                    if multiplier < 0f32 {
                        continue;
                    }

                    let biome = biomes_map.get_biome(j as i32);
                    if biome.is_none() {
                        continue;
                    }

                    let biome = biome.unwrap();
                    if biome.lake_noise_specs.seed == 0 || biome.lake_depth_multiplier == 0 {
                        continue;
                    }

                    let offset_coordinates = node.coordinates.to_offset_coordinates();
                    let lake_noise = 1f32 - biome.get_lake_noise(offset_coordinates.x as f32, offset_coordinates.z as f32);

                    if lake_noise < biome.lake_noise_threshold {
                        continue;
                    }

                    let normalized_lake_noise = inverse_lerp(biome.lake_noise_threshold, 1f32, lake_noise);
                    let smoothed_lake_noise = lerp(lake_noise, normalized_lake_noise, biome.lake_depth_smoothing);

                    multiplier_total += multiplier;
                    lake_depth_total += smoothed_lake_noise * biome.lake_depth_multiplier as f32 * multiplier;
                }

                if lake_depth_total != 0f32 {
                    node.lake_depth = (lake_depth_total / multiplier_total).round() as i16;
                }
            }
        }

        // Flatten the lake floors by setting it to the lowest possible elevation
        fn flatten_floor(graph: &mut HexGraph<TerrainNode>) {
            graph.min_flood_fill_all_areas(
                |node| node.lake_depth != 0,
                |node| node.elevation as i32,
                |node, elevation| node.elevation = elevation as i16,
            );
        }

        // Traverse the graph, apply the previously calculated lakeDepth and create barriers around the lake when needed
        fn apply_depth_and_set_water_level(biomes_map: &BiomesMapDefinition, graph: &mut HexGraph<TerrainNode>) {
            let graph_count = graph.count();

            for i in 0..graph_count {
                let node = graph.get(i).unwrap();
                let node_elevation = node.elevation;
                let node_biomes_multipliers = node.biomes_multipliers.clone();

                if node.lake_depth == 0 {
                    continue;
                }

                for j in 0..6 {
                    let neighbor_index = graph.get_neighbor(i, j);
                    if neighbor_index < 0 {
                        continue;
                    }

                    let neighbor = graph.get_mut(neighbor_index).unwrap();
                    if neighbor.lake_depth != 0 {
                        continue;
                    }

                    if neighbor.node_type == terrain_node::NodeType::Sea {
                        let mut create_lake_ocean_barrier = false;

                        for k in 0..node_biomes_multipliers.len() {
                            let multiplier = node_biomes_multipliers[k];
                            if multiplier < 0f32 {
                                continue;
                            }

                            let biome = biomes_map.get_biome(k as i32);
                            if biome.is_none() {
                                continue;
                            }

                            let biome = biome.unwrap();
                            if biome.lake_sea_barriers {
                                create_lake_ocean_barrier = true;
                                break;
                            }
                        }

                        if create_lake_ocean_barrier {
                            let elevation = node_elevation.max(neighbor.water_level);
                            neighbor.elevation = elevation;
                            neighbor.water_level = elevation;
                            continue;
                        }
                    }

                    //Prevent neighbor's water from flowing into the lake
                    if node_elevation < neighbor.water_level {
                        neighbor.elevation = neighbor.water_level;
                        continue;
                    }

                    //Prevent the lake's water from flowing into neighbors
                    if node_elevation > neighbor.elevation && node_elevation > neighbor.water_level {
                        neighbor.elevation = node_elevation;
                        neighbor.water_level = node_elevation;
                        continue;
                    }

                    //Water shader requires the WaterLevel of neighbors to be equal or a seam will be visible
                    neighbor.water_level = node_elevation;
                }

                let node = graph.get_mut(i).unwrap();
                node.water_level = node_elevation;
                node.elevation -= node.lake_depth;
                node.node_type = terrain_node::NodeType::Lake;
            }
        }
    }

    fn compute_rivers(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        let biomes_map = &world_definition.biomes_map;
        let mut lakes: Vec<Lake> = Vec::new();

        find_lakes(biomes_map, graph, &mut lakes);
        compute_rivers(biomes_map, graph, &lakes);

        // Traverse the graph looking for lake nodes that are at the border and group them by lake
        fn find_lakes(biomes_map: &BiomesMapDefinition, graph: &mut HexGraph<TerrainNode>, lakes: &mut Vec<Lake>) {
            find_lake_borders(biomes_map, graph, lakes);
            filter_lakes(lakes);

            fn find_lake_borders(biomes_map: &BiomesMapDefinition, graph: &HexGraph<TerrainNode>, lakes: &mut Vec<Lake>) {
                let mut visited: HashSet<i32> = HashSet::with_capacity(graph.count() as usize);

                for i in 0..graph.count() {
                    find_lakes(i, false, biomes_map, graph, lakes, &mut visited);
                }

                fn find_lakes(
                    node_index: i32,
                    is_recursive: bool,
                    biomes_map: &BiomesMapDefinition,
                    graph: &HexGraph<TerrainNode>,
                    lakes: &mut Vec<Lake>,
                    visited: &mut HashSet<i32>,
                ) {
                    if visited.contains(&node_index) {
                        return;
                    }

                    visited.insert(node_index);

                    let node = graph.get(node_index).unwrap();
                    let river_generation_settings = &biomes_map.get_biome(node.biome() as i32).unwrap().river_generation_settings;

                    if river_generation_settings.is_none() || !is_lake_border(&node, graph) {
                        return;
                    }

                    let river_generation_settings = river_generation_settings.as_ref().unwrap();

                    if is_recursive {
                        let last_lake = lakes.last_mut().unwrap();
                        last_lake.border_node_indices.push(node_index);
                        last_lake.min_circumference = last_lake.min_circumference.min(river_generation_settings.min_lake_circumference);
                    } else {
                        lakes.push(Lake {
                            border_node_indices: vec![node_index],
                            min_circumference: river_generation_settings.min_lake_circumference,
                        });
                    }

                    for direction in HexDirection::FLAT {
                        let neighbor_index = graph.get_index_from_hex_coordinates(node.coordinates.neighbor(direction));

                        if neighbor_index != -1 {
                            find_lakes(neighbor_index, true, biomes_map, graph, lakes, visited);
                        }
                    }

                    fn is_lake_border(node: &TerrainNode, graph: &HexGraph<TerrainNode>) -> bool {
                        // Whenever we decide what to do with existing lakes, this should become NodeType.Lake
                        if node.lake_depth == 0 {
                            return false;
                        }

                        for direction in HexDirection::FLAT {
                            let neighbor_index = graph.get_index_from_hex_coordinates(node.coordinates.neighbor(direction));

                            if neighbor_index == -1 {
                                continue;
                            }

                            let neighbor = graph.get(neighbor_index).unwrap();
                            if neighbor.lake_depth == 0 {
                                return true;
                            }
                        }

                        return false;
                    }
                }
            }

            // Remove any lakes from the list that don't meet the minimum circumference criterion
            fn filter_lakes(lakes: &mut Vec<Lake>) {
                for i in (0..lakes.len()).rev() {
                    let lake = &lakes[i];

                    if lake.border_node_indices.len() as i32 >= lake.min_circumference {
                        continue;
                    }

                    lakes.remove(i);
                }
            }
        }

        fn compute_rivers(biomes_map: &BiomesMapDefinition, graph: &mut HexGraph<TerrainNode>, lakes: &Vec<Lake>) {
            let mut rivers: Vec<River> = Vec::new();

            compute_river_combinations(graph, biomes_map, lakes, &mut rivers);
            filter_rivers(lakes, &mut rivers);
            compute_river_paths(biomes_map, graph, &mut rivers);
            apply_river_segments(biomes_map, graph, lakes, &mut rivers);

            // For each lake, create a river that connects it to every other lake and determine the shortest river length so we can use it in the minimum-spanning-tree algorithm
            fn compute_river_combinations(
                graph: &HexGraph<TerrainNode>,
                biomes_map: &BiomesMapDefinition,
                lakes: &Vec<Lake>,
                rivers: &mut Vec<River>,
            ) {
                let mut pathfinding_costs_by_lake_node_index: HashMap<i32, f32> = HashMap::new();
                get_lake_node_pathfinding_costs(graph, biomes_map, lakes, &mut pathfinding_costs_by_lake_node_index);

                for i in 0..lakes.len() {
                    for j in i + 1..lakes.len() {
                        let mut lowest_costs = f32::MAX;
                        let mut river = River::new(i, j);

                        for source_lake_node_index in &lakes[i].border_node_indices {
                            for target_lake_node_index in &lakes[j].border_node_indices {
                                let source_pathfinding_costs = pathfinding_costs_by_lake_node_index.get(source_lake_node_index);
                                let target_pathfinding_costs = pathfinding_costs_by_lake_node_index.get(target_lake_node_index);

                                if source_pathfinding_costs.is_none() || target_pathfinding_costs.is_none() {
                                    continue;
                                }

                                let source_lake_node = graph.get(*source_lake_node_index).unwrap();
                                let target_lake_node = graph.get(*target_lake_node_index).unwrap();
                                let distance = source_lake_node.coordinates.distance_to(target_lake_node.coordinates);

                                let total_costs = *source_pathfinding_costs.unwrap() + *target_pathfinding_costs.unwrap() + distance as f32;

                                if total_costs >= lowest_costs {
                                    continue;
                                }

                                lowest_costs = total_costs;
                                river.source_node_index = *source_lake_node_index;
                                river.target_node_index = *target_lake_node_index;
                                river.length = distance;
                            }
                        }

                        rivers.push(river);
                    }
                }

                // Instead of just taking the shortest path from lake to lake, we want to take the pathfinding-costs into account
                fn get_lake_node_pathfinding_costs(
                    graph: &HexGraph<TerrainNode>,
                    biomes_map: &BiomesMapDefinition,
                    lakes: &Vec<Lake>,
                    pathfinding_costs_by_lake_node_index: &mut HashMap<i32, f32>,
                ) {
                    for lake in lakes {
                        for lake_node_index in &lake.border_node_indices {
                            let lake_node = graph.get(*lake_node_index).unwrap();
                            let river_generation_settings =
                                &biomes_map.get_biome(lake_node.biome() as i32).unwrap().river_generation_settings;

                            if river_generation_settings.is_none() {
                                continue;
                            }

                            let river_generation_settings = river_generation_settings.as_ref().unwrap();

                            for direction in HexDirection::FLAT {
                                let neighbor_index = graph.get_index_from_hex_coordinates(lake_node.coordinates.neighbor(direction));

                                if neighbor_index == -1 {
                                    continue;
                                }

                                let neighbor = graph.get(neighbor_index).unwrap();
                                if neighbor.node_type != NodeType::Land {
                                    continue;
                                }

                                let elevation_difference = neighbor.elevation - lake_node.elevation;
                                let mut pathfinding_costs = river_generation_settings.get_pathfinding_costs(elevation_difference as i32);

                                if let Some(existing) = pathfinding_costs_by_lake_node_index.get(&lake_node_index) {
                                    pathfinding_costs = pathfinding_costs.min(*existing);
                                }

                                pathfinding_costs_by_lake_node_index.insert(*lake_node_index, pathfinding_costs);
                            }
                        }
                    }
                }
            }

            //Using Kruskal's algorithm for calculating a minimum-spanning-tree
            fn filter_rivers(lakes: &Vec<Lake>, rivers: &mut Vec<River>) {
                let mut hierarchy: Vec<usize> = (0..lakes.len()).collect();
                rivers.sort_by(|a, b| b.length.cmp(&a.length));

                for i in (0..rivers.len()).rev() {
                    let river = &rivers[i];
                    let source_root = find_hierarchy_root(&hierarchy, river.source_lake_index);
                    let target_root = find_hierarchy_root(&hierarchy, river.target_lake_index);

                    if source_root == target_root {
                        rivers.remove(i);
                        continue;
                    }

                    hierarchy[source_root] = target_root;
                }

                fn find_hierarchy_root(hierarchy: &Vec<usize>, node_index: usize) -> usize {
                    let mut result = node_index;

                    while result != hierarchy[result] {
                        result = hierarchy[result];
                    }

                    return result;
                }
            }

            // Run A* pathfinding that connects the two lakes, using the the RiverGenerationSettings
            fn compute_river_paths(biomes_map: &BiomesMapDefinition, graph: &HexGraph<TerrainNode>, rivers: &mut Vec<River>) {
                let mut pathfinder: Pathfinder<i32> = Pathfinder::with_capacity(1000);

                for i in (0..rivers.len()).rev() {
                    let river = rivers.get_mut(i).unwrap();
                    let source_node_limit = biomes_map
                        .get_biome(graph.get(river.source_node_index).unwrap().biome() as i32)
                        .unwrap()
                        .river_generation_settings
                        .as_ref()
                        .unwrap()
                        .pathfinding_node_limit;
                    let target_node_limit = biomes_map
                        .get_biome(graph.get(river.target_node_index).unwrap().biome() as i32)
                        .unwrap()
                        .river_generation_settings
                        .as_ref()
                        .unwrap()
                        .pathfinding_node_limit;
                    let node_limit = source_node_limit.max(target_node_limit) as usize;
                    let get_h_costs = |current: &i32| get_h_costs(graph, current, river.target_node_index);
                    let get_edges = |current: &i32| get_edges(biomes_map, graph, current, river.target_node_index);

                    let path = pathfinder.shortest_path_to_target(
                        river.source_node_index,
                        river.target_node_index,
                        get_h_costs,
                        get_edges,
                        Some(node_limit),
                    );

                    if let Some(path) = path {
                        river.path = Some(path);
                        river.elevation_by_node_index = Some(HashMap::new());
                        river.water_level_by_node_index = Some(HashMap::new());
                    }
                }

                fn get_h_costs(graph: &HexGraph<TerrainNode>, current: &i32, target: i32) -> f32 {
                    let current = graph.get(*current).unwrap();
                    let target = graph.get(target).unwrap();

                    return current.coordinates.distance_to(target.coordinates) as f32;
                }

                fn get_edges(
                    biomes_map: &BiomesMapDefinition,
                    graph: &HexGraph<TerrainNode>,
                    current: &i32,
                    target: i32,
                ) -> Vec<Edge<i32>> {
                    let mut edges: Vec<Edge<i32>> = Vec::with_capacity(6);
                    let current = graph.get(*current).unwrap();

                    for direction in HexDirection::FLAT {
                        let neighbor_index = graph.get_index_from_hex_coordinates(current.coordinates.neighbor(direction));

                        if neighbor_index == -1 {
                            continue;
                        }

                        let neighbor = graph.get(neighbor_index).unwrap();
                        if neighbor_index != target && neighbor.node_type != NodeType::Land {
                            continue;
                        }

                        let river_generation_settings = &biomes_map.get_biome(neighbor.biome() as i32).unwrap().river_generation_settings;

                        if river_generation_settings.is_none() {
                            continue;
                        }

                        let elevation_difference = neighbor.elevation - current.elevation;
                        let pathfinding_costs = 1f32.max(
                            river_generation_settings
                                .as_ref()
                                .unwrap()
                                .get_pathfinding_costs(elevation_difference as i32),
                        );

                        edges.push(Edge::new(neighbor_index, pathfinding_costs));
                    }

                    return edges;
                }
            }

            // Walk the path and apply the correct elevation and water-level using the RiverGenerationSettings
            fn apply_river_segments(
                biomes_map: &BiomesMapDefinition,
                graph: &mut HexGraph<TerrainNode>,
                lakes: &Vec<Lake>,
                rivers: &mut Vec<River>,
            ) {
                for river in rivers {
                    if river.path.is_none() {
                        continue;
                    }

                    compute_elevation_and_water_level(biomes_map, graph, river);
                    apply_elevation_and_water_level(graph, &river);
                    set_surrounding_water_level(biomes_map, graph, &river);
                    raise_lake_borders(graph, lakes);

                    // Walk the path and apply the correct elevation and water-level using the RiverGenerationSettings
                    fn compute_elevation_and_water_level(
                        biomes_map: &BiomesMapDefinition,
                        graph: &HexGraph<TerrainNode>,
                        river: &mut River,
                    ) {
                        let river_path = river.path.as_ref().unwrap();
                        let source_node = graph.get(river.source_node_index).unwrap();
                        let target_node = graph.get(river.target_node_index).unwrap();

                        for i in 1..river_path.len() - 1 {
                            let path_node = graph.get(river_path[i]).unwrap();
                            let river_generation_settings = biomes_map
                                .get_biome(path_node.biome() as i32)
                                .unwrap()
                                .river_generation_settings
                                .as_ref()
                                .unwrap();
                            let radius = river_generation_settings.radius;
                            let path_t = i as f32 / (river_path.len() as f32 - 2f32);
                            let linear_elevation = lerp(source_node.elevation as f32, target_node.elevation as f32, path_t);
                            let linear_water_level = lerp(source_node.water_level as f32, target_node.water_level as f32, path_t);
                            let mut surface_elevation = f32::MAX;

                            // Nodes around river affect surface elevation
                            for hex_coordinates in HexCoordinates::ring(path_node.coordinates, radius + 1) {
                                let node_index = graph.get_index_from_hex_coordinates(hex_coordinates);

                                if node_index == -1 {
                                    continue;
                                }

                                let node = graph.get(node_index).unwrap();
                                if node.node_type != NodeType::Land {
                                    continue;
                                }

                                surface_elevation = surface_elevation.min(node.elevation as f32);
                            }

                            // Compute water level and elevation for all river nodes
                            for j in (0..=radius).rev() {
                                let curve_t: f32 = (1f32 - j as f32 / radius as f32).max(0f32);
                                let depth_from_curve = river_generation_settings.depth_curve.evaluate(curve_t);

                                for hex_coordinates in HexCoordinates::ring(path_node.coordinates, j) {
                                    let node_index = graph.get_index_from_hex_coordinates(hex_coordinates);

                                    if node_index == -1 {
                                        continue;
                                    }

                                    let node = graph.get(node_index).unwrap();
                                    if node.node_type != NodeType::Land {
                                        continue;
                                    }

                                    add_or_update_elevation(
                                        node_index,
                                        linear_elevation,
                                        surface_elevation,
                                        depth_from_curve,
                                        river_generation_settings.erosion,
                                        river.elevation_by_node_index.as_mut().unwrap(),
                                    );
                                    add_or_update_water_level(
                                        node_index,
                                        linear_water_level,
                                        surface_elevation,
                                        river_generation_settings.erosion,
                                        river.water_level_by_node_index.as_mut().unwrap(),
                                    );
                                }
                            }
                        }

                        fn add_or_update_elevation(
                            node_index: i32,
                            linear_elevation: f32,
                            surface_elevation: f32,
                            depth_from_curve: f32,
                            erosion: f32,
                            elevation_by_node_index: &mut HashMap<i32, i16>,
                        ) {
                            let mut elevation = lerp(surface_elevation, linear_elevation, erosion);
                            elevation = elevation.min(surface_elevation);
                            elevation += depth_from_curve;
                            let mut elevation = elevation as i16;

                            if let Some(existing) = elevation_by_node_index.get(&node_index) {
                                elevation = elevation.min(*existing);
                            }

                            elevation_by_node_index.insert(node_index, elevation);
                        }

                        fn add_or_update_water_level(
                            node_index: i32,
                            linear_water_level: f32,
                            surface_elevation: f32,
                            erosion: f32,
                            water_level_by_node_index: &mut HashMap<i32, i16>,
                        ) {
                            let mut water_level = lerp(surface_elevation, linear_water_level, erosion);
                            water_level = water_level.min(surface_elevation);
                            let mut water_level = water_level as i16;

                            if let Some(existing) = water_level_by_node_index.get(&node_index) {
                                water_level = water_level.max(*existing);
                            }

                            water_level_by_node_index.insert(node_index, water_level);
                        }
                    }

                    fn apply_elevation_and_water_level(graph: &mut HexGraph<TerrainNode>, river: &River) {
                        let elevation_by_node_index = river.elevation_by_node_index.as_ref().unwrap();
                        let water_level_by_node_index = river.water_level_by_node_index.as_ref().unwrap();

                        for (node_index, elevation) in elevation_by_node_index {
                            let node = graph.get_mut(*node_index).unwrap();

                            node.elevation = *elevation;
                            node.water_level = *water_level_by_node_index.get(&node_index).unwrap();
                            node.node_type = NodeType::River;
                        }
                    }

                    //Water shader requires the WaterLevel of neighbors to be equal or a seam will be visible (an artifact due to how water shader renders columns of water)
                    fn set_surrounding_water_level(biomes_map: &BiomesMapDefinition, graph: &mut HexGraph<TerrainNode>, river: &River) {
                        let river_path = river.path.as_ref().unwrap();

                        for i in 1..river_path.len() - 1 {
                            let path_node = graph.get(river_path[i]).unwrap();
                            let water_level = path_node.water_level;
                            let radius = biomes_map
                                .get_biome(path_node.biome() as i32)
                                .unwrap()
                                .river_generation_settings
                                .as_ref()
                                .unwrap()
                                .radius;

                            for hex_coordinates in HexCoordinates::ring(path_node.coordinates, radius + 1) {
                                let node_index = graph.get_index_from_hex_coordinates(hex_coordinates);
                                if node_index == -1 {
                                    continue;
                                }

                                let node = graph.get_mut(node_index).unwrap();

                                //Skip nodes that are part of the path or already have their water-level set
                                if node.node_type != NodeType::Land || node.water_level != 0 {
                                    continue;
                                }

                                node.water_level = node.elevation.min(water_level);
                            }
                        }
                    }

                    // Raise the elevation of all river nodes that are next to a lake to one below water-level, creating a waterfall
                    fn raise_lake_borders(graph: &mut HexGraph<TerrainNode>, lakes: &Vec<Lake>) {
                        for lake in lakes {
                            for lake_node_index in &lake.border_node_indices {
                                let lake_border_node = graph.get(*lake_node_index).unwrap();
                                let lake_border_node_coordinates = lake_border_node.coordinates;
                                let lake_border_node_water_level = lake_border_node.water_level;

                                for direction in HexDirection::FLAT {
                                    let neighbor_index =
                                        graph.get_index_from_hex_coordinates(lake_border_node_coordinates.neighbor(direction));

                                    if neighbor_index == -1 {
                                        continue;
                                    }

                                    let neighbor = graph.get_mut(neighbor_index).unwrap();
                                    if neighbor.node_type == NodeType::River && neighbor.water_level > lake_border_node_water_level {
                                        neighbor.elevation = neighbor.water_level - 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn set_grass_density(world_definition: &WorldDefinition, graph: &mut HexGraph<TerrainNode>) {
        let biomes_map = &world_definition.biomes_map;

        // expand the terrain node biomes (single u8 contained in a u32) into the four highest multiplier biomes in a u32
        // the distinction is only used for grass density + biome blending

        for i in 0..graph.count() {
            let node = graph.get_mut(i).unwrap();

            let mut top_multipliers = Vec::new();

            for j in 0..node.biomes_multipliers.len() {
                if biomes_map.get_biome(j as i32).is_none() {
                    continue;
                }
                let multiplier = node.biomes_multipliers[j];
                if multiplier <= 0f32 {
                    continue;
                }

                top_multipliers.push((j, multiplier));
            }

            top_multipliers.sort_by_key(|m| 1000 - (m.1 * 1000.0) as i32);

            let values = top_multipliers.len().min(4);
            node.biomes = 0;
            for j in 0..values {
                node.biome_density += ((top_multipliers[j].1 * 128.0) as u32) << (j * 8);
                node.biomes += (top_multipliers[j].0 as u32) << (j * 8);
            }
        }
    }
}

// This one is different than in client. We receive the computed list directly from the client.
fn compute_buildings(world_definition: &WorldDefinition, graph: &mut HexGraph<EntityNode>) {
    let buildings_map = &world_definition.buildings_map;
    for building in buildings_map {
        let index = building.index;
        if index < 0 {
            continue;
        }
        let node = graph.get_mut(index).unwrap();
        node.building = Some(building.clone());
    }
}

fn is_valid_resource_footprint(
    graph: &HexGraph<EntityNode>,
    terrain_graph: &HexGraph<TerrainNode>,
    center_node: &EntityNode,
    footprint: &Vec<FootprintTile>,
    direction: HexDirection,
    resource: &ResourceDetails,
) -> bool {
    let center_coordinates = center_node.coordinates;

    for delta in footprint
        .into_iter()
        .filter(|f| f.footprint_type != (FootprintType::Perimeter) && !(f.x == 0 && f.z == 0))
    {
        let coordinates = (HexCoordinates {
            x: center_coordinates.x + delta.x,
            z: center_coordinates.z + delta.z,
            dimension: center_coordinates.dimension,
        })
        .rotate_around(&center_coordinates, (direction as i32) / 2);

        let footprint_node_index = graph.get_index_from_hex_coordinates(coordinates);
        if footprint_node_index < 0 {
            return false;
        }

        let footprint_node = graph.get(footprint_node_index).unwrap();

        if footprint_node.building.is_some() || footprint_node.resource.is_some() {
            return false;
        }

        if !footprint_node.is_valid_elevation(terrain_graph, resource) {
            return false;
        }
    }

    return true;
}

fn is_valid_resource_node(
    terrain_graph: &HexGraph<TerrainNode>,
    node: &EntityNode,
    biome_index: usize,
    resource: &ResourceDetails,
    resource_biome: &ResourceBiome,
    random: &mut CommonRNG,
) -> bool {
    if node.building.is_some() || node.resource.is_some() {
        return false;
    }
    let biome_multiplier = node.get_biome_value(biome_index, terrain_graph);
    if biome_multiplier <= 0f32 {
        //log::debug!("biome fail");
        return false;
    }
    if !node.is_valid_elevation(terrain_graph, resource) {
        return false;
    }

    let coords = node.coordinates.scale(1.0 / 3.0);
    let offset_coords = coords.to_offset_coordinates();
    let mut noise = resource_biome.get(offset_coords.x as f32, offset_coords.z as f32);
    let noise_threshold = resource_biome.noise_threshold;
    let chance = resource_biome.chance;
    if noise < noise_threshold.x || noise > noise_threshold.y {
        noise = 0f32;
    }
    noise = map(noise, noise_threshold.x, noise_threshold.y, 0f32, 1f32);
    if noise <= 0f32 {
        return false;
    }
    let place = random.bool(noise * chance * biome_multiplier);
    if !place {
        return false;
    }

    return true;
}

fn compute_resources(
    ctx: &ReducerContext,
    world_definition: &WorldDefinition,
    graph: &mut HexGraph<EntityNode>,
    terrain_graph: &HexGraph<TerrainNode>,
) {
    let default_footprint = vec![FootprintTile {
        x: 0,
        z: 0,
        footprint_type: FootprintType::Hitbox,
    }];

    let graph_count = graph.count() as usize;
    let mut taken_grid = vec![false; graph_count];
    for i in 0..graph_count {
        let node = graph.get_mut(i as i32).unwrap();
        node.resource = None;
    }

    let resources_map = &world_definition.resources_map;
    let mut map_random = CommonRNG::from_seed(resources_map.seed);

    for i in 0..resources_map.count() {
        let map_seed = map_random.i32(i32::MIN, i32::MAX);
        let resource_definition = resources_map.get_resource(i).unwrap();
        let resource = resource_definition.resource_details.clone();
        if resource.clump_id < 0 {
            continue;
        }

        for j in 0..resource_definition.count() {
            let resource_biome = resource_definition.get_biome(j).unwrap();
            let chance = resource_biome.chance;
            if chance <= 0f32 {
                continue;
            }

            let biome_index = resource_biome.biome_index;
            if biome_index < 0 {
                continue;
            }

            let seed = (Wrapping(resource_biome.noise_specs.seed) + Wrapping(map_seed)).0;
            let mut random = CommonRNG::from_seed(seed);

            let clump = ctx.db.resource_clump_desc().id().find(&resource.clump_id).unwrap();
            let footprint = clump.footprints(ctx);

            for k in 0..graph.count() {
                let node = graph.get_mut(k).unwrap().clone();

                if !is_valid_resource_node(terrain_graph, &node, biome_index as usize, &resource, &resource_biome, &mut random) {
                    continue;
                }

                let mut facing_direction = HexDirection::FLAT[map_random.usize_range(0, HexDirection::FLAT.len())];
                let mut valid = false;
                for _ in 0..HexDirection::FLAT.len() {
                    if is_valid_resource_footprint(graph, terrain_graph, &node, &footprint, facing_direction, &resource) {
                        valid = true;
                        break;
                    }

                    // If failed, rotate and try again (unless it's a single footprint centered footprint)
                    if footprint.len() == 0 || (footprint.len() == 1 && footprint[0].x == 0 && footprint[0].z == 0) {
                        break;
                    }
                    facing_direction = HexDirection::next_flat(facing_direction);
                }

                if !valid {
                    continue;
                }

                let coordinates = node.coordinates;

                for i in 0..clump.resource_id.len() {
                    let resource_id = clump.resource_id[i];
                    let offset_x = clump.x[i];
                    let offset_z = clump.z[i];
                    let resource_dir = clump.direction[i];

                    let mut res_footprint = &ctx.db.resource_desc().id().find(&resource_id).unwrap().footprint;
                    if res_footprint.len() == 0 {
                        // default: single-tile hitbox resource
                        res_footprint = &default_footprint
                    }
                    for res_footprint_delta in res_footprint.iter().filter(|f| f.footprint_type != FootprintType::Perimeter) {
                        let mut delta = res_footprint_delta.clone();
                        let is_center = res_footprint_delta.x == 0 && res_footprint_delta.z == 0;
                        delta.x += offset_x;
                        delta.z += offset_z;
                        let taken_coordinates = (HexCoordinates {
                            x: coordinates.x + delta.x,
                            z: coordinates.z + delta.z,
                            dimension: coordinates.dimension,
                        })
                        .rotate_around(&coordinates, (facing_direction as i32) / 2);
                        let taken_index = graph.get_index_from_hex_coordinates(taken_coordinates);
                        taken_grid[taken_index as usize] = true;
                        let taken_node = graph.get_mut(taken_index).unwrap();
                        let direction = if resource_dir == -1 {
                            HexDirection::FLAT[map_random.usize_range(0, HexDirection::FLAT.len())] as i32
                        } else {
                            let mut dir = facing_direction;
                            for _ in 0..resource_dir {
                                dir = HexDirection::next_flat(dir);
                            }
                            dir as i32
                        };

                        taken_node.resource = Some(ResourceData {
                            details: if is_center { Some(resource) } else { None },
                            direction,
                            resource_id: if is_center { resource_id } else { 0 },
                        });
                    }
                }
            }
        }
    }
}

fn point_on_circumference(peak_position: Vector2, p: Vector2, center: Vector2, radius: f32) -> Vector2 {
    let dp = p - peak_position;

    let a = dp.sqr_magnitude();
    if a.abs() < f32::EPSILON {
        return Vector2::negative_infinity();
    }

    let b = 2f32 * (dp.x * (peak_position.x - center.x) + dp.y * (peak_position.y - center.y));
    let mut c = center.x * center.x + center.y * center.y;
    c += peak_position.sqr_magnitude();
    c -= 2f32 * Vector2::dot(&center, &peak_position);
    c -= radius * radius;
    let bb4ac = b * b - 4f32 * a * c;
    if bb4ac < 0f32 {
        return Vector2::negative_infinity();
    }

    let mu1 = (-b + bb4ac.sqrt()) / (2f32 * a);
    let mu2 = (-b - bb4ac.sqrt()) / (2f32 * a);

    let int0 = Vector2::new(
        peak_position.x + mu1 * (p.x - peak_position.x),
        peak_position.y + mu1 * (p.y - peak_position.y),
    );
    let int1 = Vector2::new(
        peak_position.x + mu2 * (p.x - peak_position.x),
        peak_position.y + mu2 * (p.y - peak_position.y),
    );

    let d0 = int0 - p;
    let d1 = int0 - peak_position;
    return if d0.sqr_magnitude() < d1.sqr_magnitude() { int0 } else { int1 };
}
