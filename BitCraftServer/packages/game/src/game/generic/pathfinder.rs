use core::hash::Hash;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub struct Pathfinder<T>
where
    T: Eq + Hash + Clone,
{
    node_by_t: HashMap<T, Node<T>>,
    open: BinaryHeap<Node<T>>,
    closed: HashSet<T>,
}

impl<T> Default for Pathfinder<T>
where
    T: Eq + Hash + Clone,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Pathfinder<T>
where
    T: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            node_by_t: HashMap::new(),
            open: BinaryHeap::new(),
            closed: HashSet::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            node_by_t: HashMap::with_capacity(capacity),
            open: BinaryHeap::with_capacity(capacity),
            closed: HashSet::with_capacity(capacity),
        }
    }

    pub fn shortest_path_to_target(
        &mut self,
        source: T,
        target: T,
        get_h_costs: impl Fn(&T) -> f32,
        get_edges: impl Fn(&T) -> Vec<Edge<T>>,
        node_limit: Option<usize>,
    ) -> Option<Vec<T>> {
        self.clear_state();

        let node_limit = node_limit.unwrap_or(usize::MAX);
        let h_costs = get_h_costs(&source);
        let source_node = Node::new(source.clone(), None, 0f32, h_costs);
        self.open.push(source_node.clone());
        self.node_by_t.insert(source, source_node);

        while !self.open.is_empty() {
            let current = self.open.pop().unwrap();
            self.closed.insert(current.location.clone());

            if current.location == target || self.closed.len() >= node_limit {
                break;
            }

            for edge in get_edges(&current.location).iter() {
                if self.closed.contains(&edge.location) {
                    continue;
                }

                let new_g_costs = current.g_costs + edge.g_costs;

                if let Some(node) = self.node_by_t.get_mut(&edge.location) {
                    if node.g_costs > new_g_costs {
                        node.g_costs = new_g_costs;
                        node.parent = Some(current.location.clone());
                        self.open.push(node.clone());
                    }
                    continue;
                }

                let new_node = Node::new(
                    edge.location.clone(),
                    Some(current.location.clone()),
                    new_g_costs,
                    get_h_costs(&edge.location),
                );
                self.open.push(new_node.clone());
                self.node_by_t.insert(edge.location.clone(), new_node);
            }
        }

        if let Some(mut node) = self.node_by_t.get(&target) {
            let mut path: Vec<T> = Vec::new();

            while node.parent.is_some() {
                node = self.node_by_t.get(node.parent.as_ref().unwrap()).unwrap();

                path.push(node.location.clone());
            }

            if path.is_empty() {
                return None;
            }

            path.reverse();
            path.push(target);

            return Some(path);
        }

        None
    }

    fn clear_state(&mut self) {
        self.node_by_t.clear();
        self.open.clear();
        self.closed.clear();
    }
}

#[derive(Clone)]
pub struct Node<T>
where
    T: Eq + Hash,
{
    location: T,
    parent: Option<T>,
    g_costs: f32,
    h_costs: f32,
}

impl<T> Eq for Node<T> where T: Eq + Hash {}

impl<T: Eq + Hash> PartialEq for Node<T>
where
    T: Eq + Hash,
{
    fn eq(&self, other: &Self) -> bool {
        self.location.eq(&other.location)
    }
}

impl<T> Hash for Node<T>
where
    T: Eq + Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.location.hash(state);
    }
}

impl<T> Ord for Node<T>
where
    T: Eq + Hash,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed so that the BinaryHeap pops the node with the lowest f-cost first.
        other.f_costs().partial_cmp(&self.f_costs()).unwrap()
    }
}

impl<T> PartialOrd for Node<T>
where
    T: Eq + Hash,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Node<T>
where
    T: Eq + Hash,
{
    pub fn new(location: T, parent: Option<T>, g_costs: f32, h_costs: f32) -> Self {
        Self {
            location,
            parent,
            g_costs,
            h_costs,
        }
    }

    fn f_costs(&self) -> f32 {
        self.g_costs + self.h_costs
    }
}

pub struct Edge<T> {
    location: T,
    g_costs: f32,
}

impl<T> Edge<T> {
    pub fn new(location: T, g_costs: f32) -> Self {
        Self { location, g_costs }
    }
}
