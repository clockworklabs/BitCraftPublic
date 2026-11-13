use crate::game::generic::pathfinder::Edge;
use crate::game::generic::pathfinder::Pathfinder;

fn create_nodes() -> Vec<Vec<i32>> {
    let mut nodes = vec![vec![0i32; 10]; 10];
    nodes[0] = vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    nodes[1] = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    nodes[2] = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    nodes[3] = vec![0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    nodes[4] = vec![0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    nodes[5] = vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    nodes[6] = vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0];
    nodes[7] = vec![0, 0, 0, 0, 1, 0, 0, 0, 0, 0];
    nodes[8] = vec![0, 0, 0, 0, 0, 1, 1, 0, 0, 0];
    nodes[9] = vec![0, 0, 0, 0, 0, 0, 0, 1, 1, 1];

    nodes
}

fn get_h_costs(current: &(usize, usize), target: (usize, usize)) -> f32 {
    let result = (current.0 as f32 - target.0 as f32).powf(2f32) + (current.1 as f32 - target.1 as f32).powf(2f32);
    result.sqrt()
}

fn get_edges(nodes: &[Vec<i32>], current: &(usize, usize)) -> Vec<Edge<(usize, usize)>> {
    let mut edges: Vec<Edge<(usize, usize)>> = Vec::new();

    for i in -1i32..=1 {
        for j in -1i32..=1 {
            let index1 = i + current.0 as i32;
            let index2 = j + current.1 as i32;

            if index1 < 0 || index2 < 0 || index1 > 9 || index2 > 9 || (i == 0 && j == 0) {
                continue;
            }

            let index1 = index1 as usize;
            let index2 = index2 as usize;
            let neighbor = nodes[index1][index2];

            if neighbor == 1 {
                edges.push(Edge::new((index1, index2), if i == 0 || j == 0 { 1f32 } else { std::f32::consts::SQRT_2 }));
            }
        }
    }

    edges
}

#[test]
fn test_without_node_limit_path_is_some() {
    //Arrange
    let nodes = create_nodes();
    let source = (0, 0);
    let target = (9, 9);
    let get_h_costs = |current: &(usize, usize)| get_h_costs(current, target);
    let get_edges = |current: &(usize, usize)| get_edges(&nodes, current);
    let mut pathfinder: Pathfinder<(usize, usize)> = Pathfinder::new();

    //Act
    let path = pathfinder.shortest_path_to_target(source, target, get_h_costs, get_edges, None);

    //Assert
    assert!(path.is_some());
    assert_eq!(path.as_ref().unwrap().len(), 13);
    assert_eq!(path.as_ref().unwrap().first().unwrap(), &source);
    assert_eq!(path.as_ref().unwrap().last().unwrap(), &target);
}

#[test]
fn test_without_node_limit_path_is_none() {
    //Arrange
    let mut nodes = create_nodes();
    nodes[5][3] = 0;
    let source = (0, 0);
    let target = (9, 9);
    let get_h_costs = |current: &(usize, usize)| get_h_costs(current, target);
    let get_edges = |current: &(usize, usize)| get_edges(&nodes, current);
    let mut pathfinder: Pathfinder<(usize, usize)> = Pathfinder::new();

    //Act
    let path = pathfinder.shortest_path_to_target(source, target, get_h_costs, get_edges, None);

    //Assert
    assert!(path.is_none());
}

#[test]
fn test_with_node_limit_path_is_some() {
    //Arrange
    let nodes = create_nodes();
    let source = (0, 0);
    let target = (9, 9);
    let get_h_costs = |current: &(usize, usize)| get_h_costs(current, target);
    let get_edges = |current: &(usize, usize)| get_edges(&nodes, current);
    let mut pathfinder: Pathfinder<(usize, usize)> = Pathfinder::new();

    //Act
    let path = pathfinder.shortest_path_to_target(source, target, get_h_costs, get_edges, Some(20));

    //Assert
    assert!(path.is_some());
    assert_eq!(path.as_ref().unwrap().len(), 13);
    assert_eq!(path.as_ref().unwrap().first().unwrap(), &source);
    assert_eq!(path.as_ref().unwrap().last().unwrap(), &target);
}

#[test]
fn test_with_node_limit_path_is_none() {
    //Arrange
    let nodes = create_nodes();
    let source = (0, 0);
    let target = (9, 9);
    let get_h_costs = |current: &(usize, usize)| get_h_costs(current, target);
    let get_edges = |current: &(usize, usize)| get_edges(&nodes, current);
    let mut pathfinder: Pathfinder<(usize, usize)> = Pathfinder::new();

    //Act
    let path = pathfinder.shortest_path_to_target(source, target, get_h_costs, get_edges, Some(10));

    //Assert
    assert!(path.is_none());
}

fn create_nodes_map() -> Vec<Vec<i32>> {
    let mut nodes = vec![vec![0i32; 10]; 10];
    nodes[0] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    nodes[1] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    nodes[2] = vec![1, 1, 1, 1, 1, 0, 1, 1, 1, 1];
    nodes[3] = vec![1, 1, 1, 1, 1, 0, 1, 1, 1, 1];
    nodes[4] = vec![1, 1, 1, 1, 1, 0, 1, 1, 1, 1];
    nodes[5] = vec![1, 1, 0, 0, 0, 0, 1, 1, 1, 1];
    nodes[6] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    nodes[7] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    nodes[8] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    nodes[9] = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

    nodes
}

#[test]
fn test_with_open_map() {
    //Arrange
    let nodes = create_nodes_map();
    let source = (0, 0);
    let target = (9, 9);
    let get_h_costs = |current: &(usize, usize)| get_h_costs(current, target);
    let get_edges = |current: &(usize, usize)| get_edges(&nodes, current);
    let mut pathfinder: Pathfinder<(usize, usize)> = Pathfinder::new();

    //Act
    let path = pathfinder.shortest_path_to_target(source, target, get_h_costs, get_edges, None);

    //Assert
    assert!(path.is_some());
    assert_eq!(path.as_ref().unwrap().len(), 14);
    assert_eq!(path.as_ref().unwrap().first().unwrap(), &source);
    assert_eq!(path.as_ref().unwrap().last().unwrap(), &target);
}
