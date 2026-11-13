use crate::game::coordinates::{hex_coordinates::HexCoordinates, offset_coordinates::OffsetCoordinates, *};
use queues::*;
use std::collections::HashSet;

pub trait HexNode {
    fn new(x: i32, z: i32) -> Self;
    fn get_coordinates(&self) -> HexCoordinates;
}

pub struct HexGraph<T>
where
    T: HexNode,
{
    pub width: usize,
    pub depth: usize,
    pub nodes: Vec<T>,
}

impl<T> HexGraph<T>
where
    T: HexNode,
{
    pub fn new(width: usize, depth: usize) -> Self {
        let mut r = Self {
            width,
            depth,
            nodes: Vec::with_capacity(width * depth),
        };
        for z in 0..depth {
            for x in 0..width {
                r.nodes.push(T::new(x as i32, z as i32));
            }
        }
        r
    }

    pub fn count(&self) -> i32 {
        self.nodes.len() as i32
    }

    pub fn get(&self, index: i32) -> Option<&T> {
        if index < 0 || index >= self.nodes.len() as i32 {
            return None;
        }
        Some(&self.nodes[index as usize])
    }

    pub fn get_mut(&mut self, index: i32) -> Option<&mut T> {
        if index < 0 || index >= self.nodes.len() as i32 {
            return None;
        }
        Some(&mut self.nodes[index as usize])
    }

    pub fn get_index_from_node(&self, node: &T) -> i32 {
        self.get_index_from_hex_coordinates(node.get_coordinates())
    }
    pub fn get_index_from_hex_coordinates(&self, coordinates: HexCoordinates) -> i32 {
        self.get_index_from_offset_coordinates(coordinates.to_offset_coordinates())
    }
    pub fn get_index_from_offset_coordinates(&self, coordinates: OffsetCoordinates) -> i32 {
        self.get_index(coordinates.x, coordinates.z)
    }
    pub fn get_index(&self, x: i32, z: i32) -> i32 {
        if x < 0 || x >= self.width as i32 || z < 0 || z >= self.depth as i32 {
            return -1;
        }
        z * self.width as i32 + x
    }

    pub fn get_neighbor(&self, index: i32, neighbour_index: i32) -> i32 {
        if !(0..6).contains(&neighbour_index) {
            return -1;
        }

        let node = &self.nodes[index as usize];
        let coordinates = &node.get_coordinates();

        let neighbor_coordinates: HexCoordinates = match neighbour_index {
            0 => {
                coordinates.neighbor(HexDirection::NE)
            }
            1 => {
                coordinates.neighbor(HexDirection::NW)
            }
            2 => {
                coordinates.neighbor(HexDirection::W)
            }
            3 => {
                coordinates.neighbor(HexDirection::SW)
            }
            4 => {
                coordinates.neighbor(HexDirection::SE)
            }
            5 => {
                coordinates.neighbor(HexDirection::E)
            }
            _ => {
                return -1;
            }
        };

        let offset = neighbor_coordinates.to_offset_coordinates();
        self.get_index(offset.x, offset.z)
    }

    pub fn get_neighbors(&self, index: i32) -> Vec<i32> {
        let mut r = Vec::with_capacity(6);
        for i in 0..6 {
            r.push(self.get_neighbor(index, i));
        }
        r
    }

    pub fn flood_fill<V, W>(&mut self, origin: i32, predicate: V, f: W)
    where
        V: Fn(&T) -> bool,
        W: Fn(&mut T),
    {
        let mut added = HashSet::new();
        let mut pending: Queue<i32> = queue![];

        pending.add(origin).unwrap();
        added.insert(origin);

        while pending.size() > 0 {
            let node_index = pending.remove().unwrap();

            let node = &mut self.nodes[node_index as usize];
            if !predicate(node) {
                continue;
            }

            f(node);
            for i in 0..6 {
                let neighbor_index = self.get_neighbor(node_index, i);
                if neighbor_index < 0 || added.contains(&neighbor_index) {
                    continue;
                }

                added.insert(neighbor_index);
                pending.add(neighbor_index).unwrap();
            }
        }
    }

    pub fn distance_to<V, W, X>(&mut self, target: V, getter: W, setter: X)
    where
        V: Fn(&T) -> bool,
        W: Fn(&T) -> i32,
        X: Fn(&mut T, i32),
    {
        let mut pending: Queue<i32> = queue![];
        let mut added: HashSet<i32> = HashSet::new();

        for i in 0..self.nodes.len() {
            let node = &mut self.nodes[i];

            if target(node) {
                setter(node, 0);

                for j in 0..6 {
                    let neighbor_index = self.get_neighbor(i as i32, j);
                    if neighbor_index < 0 || added.contains(&neighbor_index) {
                        continue;
                    }

                    let neighbor = &self.nodes[neighbor_index as usize];

                    if target(neighbor) {
                        continue;
                    }

                    added.insert(neighbor_index);
                    pending.add(neighbor_index).unwrap();
                }
            } else {
                setter(node, -1);
            }
        }

        while pending.size() > 0 {
            let node_index = pending.remove().unwrap();

            let mut min = i32::MAX;
            for i in 0..6 {
                let neighbor_index = self.get_neighbor(node_index, i);
                if neighbor_index < 0 {
                    continue;
                }

                let neighbor = &self.nodes[neighbor_index as usize];

                let distance = getter(neighbor);
                if distance >= 0 {
                    if distance < min {
                        min = distance;
                    }
                } else if !added.contains(&neighbor_index) {
                    added.insert(neighbor_index);
                    pending.add(neighbor_index).unwrap();
                }
            }

            let node = &mut self.nodes[node_index as usize];
            setter(node, min + 1);
        }
    }

    /// Find all areas that satisfy `predicate` and flood-fill each one with a LOCAL minimum value
    pub fn min_flood_fill_all_areas<V, W, X>(&mut self, predicate: V, getter: W, setter: X)
    where
        V: Fn(&T) -> bool,
        W: Fn(&T) -> i32,
        X: Fn(&mut T, i32),
    {
        let nodes_count = self.nodes.len();
        let mut filtered_nodes: HashSet<i32> = HashSet::with_capacity(nodes_count / 2);
        for index in 0..nodes_count {
            if !predicate(&self.nodes[index]) {
                continue;
            }
            filtered_nodes.insert(index as i32);
        }

        let mut contiguous_area: Vec<i32> = Vec::with_capacity(filtered_nodes.len() / 2);
        let mut open: Queue<i32> = Queue::new();
        while !filtered_nodes.is_empty() {
            contiguous_area.clear();

            let mut index = *filtered_nodes.iter().next().unwrap();
            filtered_nodes.remove(&index);
            contiguous_area.push(index);
            open.add(index).unwrap();

            let mut min = getter(&self.nodes[index as usize]);
            while open.peek().is_ok() {
                index = open.remove().unwrap();
                for neighbor in self.get_neighbors(index) {
                    if filtered_nodes.contains(&neighbor) {
                        filtered_nodes.remove(&neighbor);
                        open.add(neighbor).unwrap();
                        contiguous_area.push(neighbor);
                        min = min.min(getter(&self.nodes[neighbor as usize]));
                    }
                }
            }

            for &node_index in contiguous_area.iter() {
                setter(&mut self.nodes[node_index as usize], min);
            }
        }
    }
}
