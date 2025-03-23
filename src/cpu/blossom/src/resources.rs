// see micro-blossom/resources/graphs/README.md

use fusion_blossom::example_codes::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use mwmatching::Matching;
use ordered_float::OrderedFloat;
use petgraph::{algo::floyd_warshall, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicroBlossomSingle {
    /// only for visualization positions of the vertices
    pub positions: Vec<Position>,
    /// number of vertices, including virtual vertices
    pub vertex_num: usize,
    /// each edge is a tuple of (left vertex, right vertex, weight)
    pub weighted_edges: Vec<WeightedEdge>,
    /// virtual vertices that corresponds to the boundary of the code, can be matched arbitrary times
    pub virtual_vertices: Vec<usize>,
    /// a binary tree from every vertex to a single root, for communication with CPU
    pub vertex_binary_tree: BinaryTree,
    /// a binary tree from every edge to a single root, for communication with CPU
    pub edge_binary_tree: BinaryTree,
    /// a combined binary tree from vertex and edge, for communication with CPU
    pub vertex_edge_binary_tree: BinaryTree, // first vertex, then edge
    /// maximum growth of each vertex, should be set to the maximum possible length to the nearest virtual vertex;
    /// if simplicity is desired, set it to sum_{e \in E} w_e would suffice.
    pub vertex_max_growth: Vec<isize>,
    /// primal offloading, with three types: regular edge, virtual edge and fusion edge
    pub offloading: OffloadingFinder,
    /// round-wise fusion, supposedly layer by layer but can be customized
    pub layer_fusion: Option<LayerFusion>,
    /// parity tracker allows the hardware to report the pre-matched result;
    /// when combining with the parity result from the CPU, it constitutes the complete decoded result.
    /// an arbitrary number of parity reports is supported, but they are not allowed to change
    /// in runtime in the current implementation.
    pub parity_reporters: Option<ParityReporters>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub i: f64,
    pub j: f64,
    /// time axis, pointing upwards
    pub t: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedEdge {
    /// left vertex
    pub l: usize,
    /// right vertex
    pub r: usize,
    /// weight
    pub w: isize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryTreeNode {
    /// parent if exists
    #[serde(rename = "p")]
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<usize>,
    /// left and right children
    #[serde(rename = "l")]
    #[serde(skip_serializing_if = "Option::is_none")]
    left: Option<usize>,
    #[serde(rename = "r")]
    #[serde(skip_serializing_if = "Option::is_none")]
    right: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryTree {
    /// the number of nodes is equal to the number of elements in the binary tree
    nodes: Vec<BinaryTreeNode>,
}

/// the type of offloading
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OffloadingType {
    /// a pair of defects match with each other (regular edge)
    #[serde(rename = "dm")]
    DefectMatch {
        #[serde(rename = "e")]
        edge_index: usize,
    },
    /// a defect match with virtual vertex (boundary edge)
    #[serde(rename = "vm")]
    VirtualMatch {
        #[serde(rename = "e")]
        edge_index: usize,
        #[serde(rename = "v")]
        virtual_vertex: usize,
    },
    /// temporary match with fusion boundary that is guaranteed to be removed in the end
    #[serde(rename = "fm")]
    FusionMatch {
        #[serde(rename = "e")]
        edge_index: usize,
        #[serde(rename = "c")]
        conditioned_vertex: usize,
    },
}

impl MicroBlossomSingle {
    pub fn new(initializer: &SolverInitializer, positions: &[VisualizePosition]) -> Self {
        let positions: Vec<_> = positions.iter().map(|p| Position { t: p.t, i: p.i, j: p.j }).collect();
        let weighted_edges: Vec<_> = initializer
            .weighted_edges
            .iter()
            .map(|e| WeightedEdge {
                l: e.0.try_into().unwrap(),
                r: e.1.try_into().unwrap(),
                w: e.2,
            })
            .collect();
        // construct vertex and edge binary tree with geometric distance information
        let vertex_binary_tree = BinaryTree::inferred_from_positions(&positions);
        let mut edge_positions = vec![];
        for edge in weighted_edges.iter() {
            let left = &positions[edge.l as usize];
            let right = &positions[edge.r as usize];
            edge_positions.push(Position {
                i: f64::min(left.i, right.i),
                j: f64::min(left.j, right.j),
                t: f64::min(left.t, right.t),
            })
        }
        let edge_binary_tree = BinaryTree::inferred_from_positions(&edge_positions);
        let vertex_edge_positions: Vec<_> = positions.iter().chain(edge_positions.iter()).cloned().collect();
        let vertex_edge_binary_tree = BinaryTree::inferred_from_positions(&vertex_edge_positions);
        let vertex_max_growth = infer_vertex_max_growth(initializer);
        let mut offloading = OffloadingFinder::new();
        offloading.find_first_order(initializer);
        let mut result = Self {
            vertex_num: initializer.vertex_num.try_into().unwrap(),
            positions,
            weighted_edges,
            virtual_vertices: initializer.virtual_vertices.clone(),
            vertex_binary_tree,
            edge_binary_tree,
            vertex_edge_binary_tree,
            vertex_max_growth,
            offloading,
            layer_fusion: None,
            parity_reporters: None,
        };
        result.layer_fusion = Some(LayerFusion::new(&result));
        result
    }

    pub fn new_code(code: &dyn ExampleCode) -> Self {
        let initializer = code.get_initializer();
        let positions = code.get_positions();
        assert_eq!(positions.len(), initializer.vertex_num as usize);
        Self::new(&initializer, &positions)
    }

    /// warning: do not use this for production because it doesn't contain useful position information
    /// to ease timing when placing on the hardware; only use this for behavior simulation
    pub fn new_initializer_only(initializer: &SolverInitializer) -> Self {
        let positions: Vec<VisualizePosition> = (0..initializer.vertex_num)
            .map(|_| VisualizePosition::new(0., 0., 0.))
            .collect();
        Self::new(initializer, &positions)
    }

    pub fn get_initializer(&self) -> SolverInitializer {
        SolverInitializer::new(
            self.vertex_num,
            self.weighted_edges.iter().map(|edge| (edge.l, edge.r, edge.w)).collect(),
            self.virtual_vertices.clone(),
        )
    }

    pub fn get_positions(&self) -> Vec<VisualizePosition> {
        self.positions
            .iter()
            .map(|position| VisualizePosition::new(position.i, position.j, position.t))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OffloadingFinder(pub Vec<OffloadingType>);

impl OffloadingFinder {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn find_first_order(&mut self, initializer: &SolverInitializer) {
        self.find_defect_match(initializer);
        self.find_virtual_match(initializer);
    }

    pub fn find_defect_match(&mut self, initializer: &SolverInitializer) {
        let virtual_vertices: BTreeSet<_> = initializer.virtual_vertices.iter().cloned().collect();
        for (edge_index, (l, r, _weight)) in initializer.weighted_edges.iter().enumerate() {
            let is_virtual_left = virtual_vertices.contains(l);
            let is_virtual_right = virtual_vertices.contains(r);
            if !is_virtual_left && !is_virtual_right {
                self.0.push(OffloadingType::DefectMatch { edge_index })
            }
        }
    }

    pub fn find_virtual_match(&mut self, initializer: &SolverInitializer) {
        let virtual_vertices: BTreeSet<_> = initializer.virtual_vertices.iter().cloned().collect();
        for (edge_index, (l, r, _weight)) in initializer.weighted_edges.iter().enumerate() {
            let is_virtual_left = virtual_vertices.contains(l);
            let is_virtual_right = virtual_vertices.contains(r);
            if is_virtual_left {
                self.0.push(OffloadingType::VirtualMatch {
                    edge_index,
                    virtual_vertex: *l,
                })
            }
            if is_virtual_right {
                self.0.push(OffloadingType::VirtualMatch {
                    edge_index,
                    virtual_vertex: *r,
                })
            }
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
struct Coordinate2D {
    i: i64,
    j: i64,
}

impl Coordinate2D {
    fn new(position: &Position) -> Self {
        Self {
            i: position.i.round() as i64,
            j: position.j.round() as i64,
        }
    }

    fn manhattan_distance_to(&self, other: &Self) -> i64 {
        (self.i - other.i).abs() + (self.j - other.j).abs()
    }
}

impl BinaryTreeNode {
    pub fn new() -> Self {
        Self {
            parent: None,
            left: None,
            right: None,
        }
    }

    fn has_child(&self, child_index: usize) -> bool {
        self.left == Some(child_index) || self.right == Some(child_index)
    }
}

impl BinaryTree {
    pub fn new(leaf_nodes: usize) -> Self {
        let mut tree = Self { nodes: vec![] };
        for _ in 0..leaf_nodes {
            tree.nodes.push(BinaryTreeNode::new())
        }
        tree
    }

    pub fn inferred_from_positions(positions: &[Position]) -> Self {
        let mut tree = Self::new(positions.len());
        if positions.len() < 2 {
            return tree;
        }
        // build a subtree for each element at the same (i, j) coordinates;
        // they are supposed to belong to the same stabilizer measurement
        let mut stab_subtrees: BTreeMap<Coordinate2D, Vec<usize>> = BTreeMap::new();
        for (vertex_index, position) in positions.iter().enumerate() {
            let stab_id = Coordinate2D::new(position);
            stab_subtrees.entry(stab_id).or_insert_with(Vec::new).push(vertex_index);
        }
        // construct the roots of the subtrees
        let mut subtree_roots: Vec<(Coordinate2D, usize)> = vec![];
        for (coordinate, mut subtree) in stab_subtrees.into_iter() {
            // construct a fat binary tree
            assert!(!subtree.is_empty());
            while subtree.len() > 1 {
                let mut new_subtree = vec![];
                for idx in 0..subtree.len() / 2 {
                    let node_index = tree.nodes.len();
                    new_subtree.push(node_index);
                    let mut tree_node = BinaryTreeNode::new();
                    let left = subtree[2 * idx];
                    let right = subtree[2 * idx + 1];
                    tree_node.left = Some(left);
                    tree_node.right = Some(right);
                    debug_assert!(tree.nodes[left].parent.is_none());
                    debug_assert!(tree.nodes[right].parent.is_none());
                    tree.nodes[left].parent = Some(node_index);
                    tree.nodes[right].parent = Some(node_index);
                    tree.nodes.push(tree_node);
                }
                if subtree.len() % 2 == 1 {
                    new_subtree.push(*subtree.last().unwrap());
                }
                subtree = new_subtree;
            }
            subtree_roots.push((coordinate, subtree[0]));
        }
        // then construct a max-cardinality matching between the roots using geometric distance
        while subtree_roots.len() > 1 {
            let matching = find_max_cardinality_matching_with_minimum_weight(&subtree_roots);
            let mut matched = vec![false; subtree_roots.len()];
            let mut new_subtree_roots = vec![];
            for (i, j) in matching.into_iter() {
                assert_eq!(matched[i], false);
                assert_eq!(matched[j], false);
                matched[i] = true;
                matched[j] = true;
                let left = subtree_roots[i].1;
                let right = subtree_roots[j].1;
                let node_index = tree.nodes.len();
                new_subtree_roots.push((
                    Coordinate2D {
                        i: (subtree_roots[i].0.i + subtree_roots[j].0.i) / 2,
                        j: (subtree_roots[i].0.j + subtree_roots[j].0.j) / 2,
                    },
                    node_index,
                ));
                let mut tree_node = BinaryTreeNode::new();
                tree_node.left = Some(left);
                tree_node.right = Some(right);
                debug_assert!(tree.nodes[left].parent.is_none());
                debug_assert!(tree.nodes[right].parent.is_none());
                tree.nodes[left].parent = Some(node_index);
                tree.nodes[right].parent = Some(node_index);
                tree.nodes.push(tree_node);
            }
            for (value, is_matched) in subtree_roots.into_iter().zip(matched.iter()) {
                if !is_matched {
                    new_subtree_roots.push(value);
                }
            }
            subtree_roots = new_subtree_roots;
        }
        tree.sanity_check(positions);
        tree
    }

    fn sanity_check(&self, positions: &[Position]) {
        assert_eq!(self.nodes.len(), positions.len() * 2 - 1);
        if positions.len() > 1 {
            for (i, tree_node) in self.nodes.iter().enumerate() {
                if i == self.nodes.len() - 1 {
                    assert!(tree_node.parent.is_none());
                } else {
                    assert!(tree_node.parent.is_some());
                    assert!(self.nodes[tree_node.parent.unwrap()].has_child(i));
                }
                if i < positions.len() {
                    assert!(tree_node.left.is_none());
                    assert!(tree_node.right.is_none());
                } else {
                    assert!(tree_node.left.is_some());
                    assert_eq!(self.nodes[tree_node.left.unwrap()].parent, Some(i));
                    assert!(tree_node.right.is_some());
                    assert_eq!(self.nodes[tree_node.right.unwrap()].parent, Some(i));
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerFusion {
    pub num_layers: usize,
    /// `layers[layer_id] = Vec<vertices>`
    pub layers: Vec<Vec<usize>>,
    /// mapping from vertex index to layer id
    pub vertex_layer_id: BTreeMap<usize, usize>,
    /// mapping from edge index to the conditioned vertex index
    pub fusion_edges: BTreeMap<usize, usize>,
    /// mapping from vertex index to the conditioned edge indices
    pub unique_tight_conditions: BTreeMap<usize, Vec<usize>>,
}

impl LayerFusion {
    /// automatically infer fusion plan according to the position
    pub fn new(graph: &MicroBlossomSingle) -> Self {
        // useful for observing fusion when the graph is 2D
        let mut demo_2d_fusion = false;
        {
            let t_values: BTreeSet<OrderedFloat<f64>> = graph.positions.iter().map(|pos| pos.t.into()).collect();
            if t_values.len() == 1 {
                demo_2d_fusion = true;
            }
        }
        let get_t = if !demo_2d_fusion {
            |position: &Position| position.t
        } else {
            |position: &Position| -position.i
        };
        // first find all t position values
        let mut layers = BTreeMap::<OrderedFloat<f64>, Vec<usize>>::new();
        let virtual_vertices: BTreeSet<usize> = graph.virtual_vertices.iter().cloned().collect();
        for (vertex_index, position) in graph.positions.iter().enumerate() {
            if virtual_vertices.contains(&vertex_index) {
                continue; // do not count virtual vertices
            }
            let t = get_t(position).into();
            if let Some(layer) = layers.get_mut(&t) {
                layer.push(vertex_index);
            } else {
                layers.insert(t, vec![vertex_index]);
            }
        }
        let mut vertex_layer_id = BTreeMap::<usize, usize>::new(); // vertex_index: layer id
        for (layer_id, vertices) in layers.values().enumerate() {
            for vertex_index in vertices.iter() {
                vertex_layer_id.insert(*vertex_index, layer_id);
            }
        }
        // iterate every edge and see if they should have conditioned half weight
        let mut fusion_edges = BTreeMap::<usize, usize>::new(); // edge_index: conditioned_vertex_index
        let mut unique_tight_conditions = BTreeMap::<usize, Vec<usize>>::new(); // vertex_index: conditioned edges
        for (edge_index, edge) in graph.weighted_edges.iter().enumerate() {
            if virtual_vertices.contains(&edge.l) || virtual_vertices.contains(&edge.r) {
                continue; // ignore if one of them is virtual
            }
            let left_layer_id = vertex_layer_id[&edge.l];
            let right_layer_id = vertex_layer_id[&edge.r];
            if left_layer_id == right_layer_id {
                continue; // ignore if they are in the same layer
            }
            let (early_vertex_index, late_vertex_index) = if left_layer_id < right_layer_id {
                (edge.l, edge.r)
            } else {
                (edge.r, edge.l)
            };
            fusion_edges.insert(edge_index, late_vertex_index);
            if let Some(conditioned_edges) = unique_tight_conditions.get_mut(&early_vertex_index) {
                conditioned_edges.push(edge_index);
            } else {
                unique_tight_conditions.insert(early_vertex_index, vec![edge_index]);
            }
        }
        Self {
            num_layers: layers.len(),
            layers: layers.values().cloned().collect(),
            vertex_layer_id,
            fusion_edges,
            unique_tight_conditions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityReporters {
    /// a reporter the XOR of multiple offloader; there could be multiple reporters
    pub reporters: Vec<Vec<usize>>,
}

impl ParityReporters {
    pub fn new() -> Self {
        Self { reporters: vec![] }
    }

    pub fn add_parity_reporter(&mut self, offloaders: Vec<usize>) {
        self.reporters.push(offloaders);
    }
}

fn find_max_cardinality_matching_with_minimum_weight(vertices: &Vec<(Coordinate2D, usize)>) -> Vec<(usize, usize)> {
    assert!(vertices.len() > 1, "should not call this function with less than 2 vertices");
    let mut matching = vec![];
    let mut edges = vec![];
    for i in 0..vertices.len() - 1 {
        for j in i + 1..vertices.len() {
            let distance = vertices[i].0.manhattan_distance_to(&vertices[j].0);
            edges.push((i, j, -distance as i32)); // we want to minimize distance
        }
    }
    let mates = Matching::new(edges).max_cardinality().solve();
    for (i, j) in mates.into_iter().enumerate() {
        if i < j && j != usize::MAX {
            matching.push((i, j));
        }
    }
    matching
}

fn infer_vertex_max_growth(initializer: &SolverInitializer) -> Vec<isize> {
    let mut max_growth = vec![];
    let mut graph = UnGraph::<usize, isize>::new_undirected();
    let node_indices: Vec<_> = (0..initializer.vertex_num)
        .map(|vertex_index| graph.add_node(vertex_index))
        .collect();
    for &(l, r, w) in initializer.weighted_edges.iter() {
        graph.add_edge(node_indices[l], node_indices[r], w);
    }
    let distance = floyd_warshall(&graph, |edge| *edge.weight()).unwrap();
    let mut is_virtual = vec![false; initializer.vertex_num];
    for &vertex_index in initializer.virtual_vertices.iter() {
        is_virtual[vertex_index] = true;
    }
    for i in 0..initializer.vertex_num {
        let mut nearest_virtual = isize::MAX;
        let mut farthest_non_virtual = 0;
        for j in 0..initializer.vertex_num {
            let dij = *distance.get(&(node_indices[i], node_indices[j])).unwrap();
            if is_virtual[j] {
                nearest_virtual = std::cmp::min(nearest_virtual, dij);
            } else {
                farthest_non_virtual = std::cmp::max(farthest_non_virtual, dij);
            }
        }
        max_growth.push(if nearest_virtual == isize::MAX {
            farthest_non_virtual
        } else {
            nearest_virtual
        });
    }
    max_growth
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resources_micro_blossom_test_1() {
        // cargo test resources_micro_blossom_test_1 -- --nocapture
        let code = CodeCapacityRepetitionCode::new(3, 0.1, 500);
        let micro_blossom = MicroBlossomSingle::new_code(&code);
        println!("micro_blossom: {micro_blossom:?}");
    }

    /// test phenomenological
    #[test]
    fn resources_micro_blossom_test_2() {
        // cargo test resources_micro_blossom_test_2 -- --nocapture
        let code = PhenomenologicalRotatedCode::new(5, 5, 0.1, 500);
        let micro_blossom = MicroBlossomSingle::new_code(&code);
        println!("micro_blossom: {micro_blossom:?}");
    }

    #[test]
    fn resources_max_cardinality_matching() {
        // cargo test resources_max_cardinality_matching -- --nocapture
        let edges = vec![(0, 3, -10), (3, 2, -20), (2, 1, -10), (3, 4, -100)];
        let mates = Matching::new(edges).max_cardinality().solve();
        assert_eq!(mates, [3, 2, 1, 0, usize::MAX]);
    }

    #[test]
    fn resources_micro_blossom_fusion_plan_1() {
        // cargo test resources_micro_blossom_fusion_plan_1 -- --nocapture
        let visualize_filename = "resources_micro_blossom_fusion_plan_1.json".to_string();
        let mut code = PhenomenologicalRotatedCode::new(3, 3, 0.1, 500);
        let micro_blossom = MicroBlossomSingle::new_code(&code);
        println!("{:?}", micro_blossom.layer_fusion);
        visualize_code(&mut code, visualize_filename);
    }

    #[test]
    fn resources_micro_blossom_fusion_plan_2() {
        // cargo test resources_micro_blossom_fusion_plan_2 -- --nocapture
        let visualize_filename = "resources_micro_blossom_fusion_plan_2.json".to_string();
        let d = 3;
        let config = json!({
            "qubit_type": qecp::types::QubitType::StabZ,
            "max_half_weight": 7,
            "nm": d-1,  // d-1 noisy measurement rounds and 1 perfect measurement rounds
        });
        let mut code = crate::example_codes::QECPlaygroundCode::new(d, 0.001, config);
        let micro_blossom = MicroBlossomSingle::new_code(&code);
        println!("{:?}", micro_blossom.layer_fusion);
        visualize_code(&mut code, visualize_filename);
    }
}
