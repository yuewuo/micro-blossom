//! Register Transfer Level (RTL) Dual Module
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::blossom_tracker::*;
use micro_blossom_nostd::util::*;
use serde_json::json;

#[derive(Debug)]
pub struct DualModuleRTL {
    // always reconstruct the whole graph when reset
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub nodes: Vec<DualNodePtr>,
    pub blossom_tracker: Box<BlossomTracker<MAX_NODE_NUM>>,
    /// temporary list of synchronize requests, not used until hardware fusion
    pub sync_requests: Vec<SyncRequest>,
}

#[derive(Debug)]
pub enum Instruction {
    SetSpeed { node: NodeIndex, speed: DualNodeGrowState },
    SetBlossom { node: NodeIndex, blossom: NodeIndex },
    AddDefectVertex { vertex: VertexIndex, node: NodeIndex },
    FindObstacle { region_preference: usize },
    Grow { length: Weight },
}

#[derive(Debug)]
pub enum Response {
    NonZeroGrow {
        length: Weight,
    },
    Conflict {
        node_1: NodeIndex,
        node_2: NodeIndex,
        touch_1: NodeIndex,
        touch_2: NodeIndex,
        vertex_1: VertexIndex,
        vertex_2: VertexIndex,
    },
    BlossomNeedExpand {
        blossom: NodeIndex,
    },
}

const VIRTUAL_NODE_INDEX: NodeIndex = NodeIndex::MAX;

impl Response {
    pub fn reduce(resp1: Option<Response>, resp2: Option<Response>) -> Option<Response> {
        if resp1.is_none() {
            return resp2;
        }
        if resp2.is_none() {
            return resp1;
        }
        let resp1 = resp1.unwrap();
        let resp2 = resp2.unwrap();
        if !matches!(resp1, Response::NonZeroGrow { .. }) {
            return Some(resp1);
        }
        if !matches!(resp2, Response::NonZeroGrow { .. }) {
            return Some(resp2);
        }
        let Response::NonZeroGrow { length: length1 } = resp1 else { unreachable!() };
        let Response::NonZeroGrow { length: length2 } = resp2 else { unreachable!() };
        Some(Response::NonZeroGrow {
            length: std::cmp::min(length1, length2),
        })
    }
}

pub fn get_blossom_roots(dual_node_ptr: &DualNodePtr) -> Vec<NodeIndex> {
    let node = dual_node_ptr.read_recursive();
    match &node.class {
        DualNodeClass::Blossom { nodes_circle, .. } => {
            let mut node_indices = vec![];
            for node_ptr in nodes_circle.iter() {
                node_indices.append(&mut get_blossom_roots(&node_ptr.upgrade_force()));
            }
            node_indices
        }
        DualNodeClass::DefectVertex { .. } => vec![node.index],
    }
}

impl DualModuleImpl for DualModuleRTL {
    fn new_empty(initializer: &SolverInitializer) -> Self {
        let mut dual_module = DualModuleRTL {
            initializer: initializer.clone(),
            vertices: vec![],
            edges: vec![],
            nodes: vec![],
            blossom_tracker: Box::new(BlossomTracker::new()),
            sync_requests: vec![],
        };
        dual_module.clear();
        dual_module
    }

    fn clear(&mut self) {
        // set vertices
        self.vertices = (0..self.initializer.vertex_num)
            .map(|vertex_index| Vertex {
                vertex_index,
                edge_indices: vec![],
                speed: DualNodeGrowState::Stay,
                grown: 0,
                is_virtual: false,
                is_defect: false,
                node_index: None,
                root_index: None,
                shadow_node_index: None,
                shadow_root_index: None,
                shadow_speed: DualNodeGrowState::Stay,
            })
            .collect();
        // set virtual vertices
        for &virtual_vertex in self.initializer.virtual_vertices.iter() {
            self.vertices[virtual_vertex].is_virtual = true;
            self.vertices[virtual_vertex].node_index = Some(VIRTUAL_NODE_INDEX);
            self.vertices[virtual_vertex].root_index = Some(VIRTUAL_NODE_INDEX);
        }
        // set edges
        self.edges.clear();
        for (edge_index, &(i, j, weight)) in self.initializer.weighted_edges.iter().enumerate() {
            self.edges.push(Edge {
                edge_index,
                weight,
                left_index: i,
                right_index: j,
                left_growth: 0,
                right_growth: 0,
            });
            for vertex_index in [i, j] {
                self.vertices[vertex_index].edge_indices.push(edge_index);
            }
        }
        // each vertex must have at least one incident edge
        for vertex in self.vertices.iter() {
            assert!(!vertex.edge_indices.is_empty());
        }
        // clear nodes
        self.nodes.clear();
        // clear tracker
        self.blossom_tracker.clear();
    }

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let node = dual_node_ptr.read_recursive();
        assert_eq!(node.index, self.nodes.len());
        self.nodes.push(dual_node_ptr.clone());
        match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => {
                self.blossom_tracker
                    .create_blossom(micro_blossom_nostd::util::ni!(node.index));
                // creating blossom is cheap
                for weak_ptr in nodes_circle.iter() {
                    let child_node_ptr = weak_ptr.upgrade_force();
                    let child_node = child_node_ptr.read_recursive();
                    self.execute_instruction(Instruction::SetBlossom {
                        node: child_node.index,
                        blossom: node.index,
                    });
                    if matches!(child_node.class, DualNodeClass::Blossom { .. }) {
                        self.blossom_tracker
                            .set_speed(micro_blossom_nostd::util::ni!(child_node.index), CompactGrowState::Stay);
                    }
                }
                // TODO: use priority queue to track shrinking blossom constraint
            }
            DualNodeClass::DefectVertex { defect_index } => {
                assert!(!self.vertices[*defect_index].is_defect, "cannot set defect twice");
                self.execute_instruction(Instruction::AddDefectVertex {
                    vertex: *defect_index,
                    node: node.index,
                });
            }
        }
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        // remove blossom is expensive because the vertices doesn't remember all the chain of blossom
        let node = dual_node_ptr.read_recursive();
        let nodes_circle = match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => nodes_circle.clone(),
            _ => unreachable!(),
        };
        self.blossom_tracker
            .set_speed(micro_blossom_nostd::util::ni!(node.index), CompactGrowState::Stay);
        for weak_ptr in nodes_circle.iter() {
            let node_ptr = weak_ptr.upgrade_force();
            let roots = get_blossom_roots(&node_ptr);
            let blossom_index = node_ptr.read_recursive().index;
            for &root_index in roots.iter() {
                self.execute_instruction(Instruction::SetBlossom {
                    node: root_index,
                    blossom: blossom_index,
                });
            }
        }
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        let node = dual_node_ptr.read_recursive();
        let node_index = dual_node_ptr.read_recursive().index;
        if matches!(node.class, DualNodeClass::Blossom { .. }) {
            self.blossom_tracker.set_speed(
                micro_blossom_nostd::util::ni!(node_index),
                match grow_state {
                    DualNodeGrowState::Grow => CompactGrowState::Grow,
                    DualNodeGrowState::Shrink => CompactGrowState::Shrink,
                    DualNodeGrowState::Stay => CompactGrowState::Stay,
                },
            );
        }
        self.execute_instruction(Instruction::SetSpeed {
            node: node_index,
            speed: grow_state,
        });
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        let return_value = self
            .execute_instruction(Instruction::FindObstacle { region_preference: 0 })
            .unwrap();
        let maximum_growth_blossom_hit_zero = self.blossom_tracker.get_maximum_growth();
        let mut max_update_length = match return_value {
            Response::NonZeroGrow { mut length } => {
                if let Some((blossom_length, _)) = &maximum_growth_blossom_hit_zero {
                    let blossom_length: Weight = (*blossom_length).try_into().unwrap();
                    length = std::cmp::min(length, blossom_length);
                }
                MaxUpdateLength::NonZeroGrow((length, false))
            }
            Response::Conflict {
                node_1,
                node_2,
                touch_1,
                touch_2,
                vertex_1,
                vertex_2,
            } => {
                if node_1 != VIRTUAL_NODE_INDEX && node_2 != VIRTUAL_NODE_INDEX {
                    MaxUpdateLength::Conflicting(
                        (self.nodes[node_1].clone(), self.nodes[touch_1].clone()),
                        (self.nodes[node_2].clone(), self.nodes[touch_2].clone()),
                    )
                } else {
                    assert!(node_1 != VIRTUAL_NODE_INDEX || node_2 != VIRTUAL_NODE_INDEX);
                    let (node, touch, virtual_vertex) = if node_1 != VIRTUAL_NODE_INDEX {
                        (node_1, touch_1, vertex_2)
                    } else {
                        (node_2, touch_2, vertex_1)
                    };
                    MaxUpdateLength::TouchingVirtual(
                        (self.nodes[node].clone(), self.nodes[touch].clone()),
                        (virtual_vertex, false),
                    )
                }
            }
            Response::BlossomNeedExpand { blossom } => MaxUpdateLength::BlossomNeedExpand(self.nodes[blossom].clone()),
        };
        // get blossom expand event from blossom tracker, only when no other conflicts are detected
        if matches!(max_update_length, MaxUpdateLength::NonZeroGrow { .. }) {
            if let Some((length, blossom_index)) = maximum_growth_blossom_hit_zero {
                if length == 0 {
                    max_update_length = MaxUpdateLength::BlossomNeedExpand(self.nodes[blossom_index.get() as usize].clone());
                }
            }
        }
        if let MaxUpdateLength::NonZeroGrow((length, _)) = &max_update_length {
            assert!(*length > 0);
        }
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        group_max_update_length.add(max_update_length);
        group_max_update_length
    }

    fn grow(&mut self, length: Weight) {
        assert!(length > 0, "RTL design doesn't allow negative growth");
        self.blossom_tracker.advance_time(length.try_into().unwrap());
        self.execute_instruction(Instruction::Grow { length });
    }

    fn prepare_nodes_shrink(&mut self, _nodes_circle: &[DualNodePtr]) -> &mut Vec<SyncRequest> {
        self.sync_requests.clear();
        &mut self.sync_requests
    }
}

macro_rules! pipeline_staged {
    ($dual_module:ident, $instruction:ident, $stage_name:ident) => {
        let vertices_next = $dual_module
            .vertices
            .iter()
            .cloned()
            .map(|mut vertex| {
                vertex.$stage_name($dual_module, &$instruction);
                vertex
            })
            .collect();
        let edges_next = $dual_module
            .edges
            .iter()
            .cloned()
            .map(|mut edge| {
                edge.$stage_name($dual_module, &$instruction);
                edge
            })
            .collect();
        $dual_module.vertices = vertices_next;
        $dual_module.edges = edges_next;
    };
}

impl DualModuleRTL {
    fn execute_instruction(&mut self, instruction: Instruction) -> Option<Response> {
        pipeline_staged!(self, instruction, execute_stage);
        pipeline_staged!(self, instruction, update_stage);
        self.vertices
            .iter()
            .map(|vertex| vertex.write_stage(self, &instruction))
            .chain(self.edges.iter().map(|edge| edge.write_stage(self, &instruction)))
            .reduce(Response::reduce)
            .unwrap()
    }
}

pub trait DualPipelined {
    /// load data from BRAM (optional)
    fn load_stage(&mut self, _dual_module: &DualModuleRTL, _instruction: &Instruction) {}
    /// execute growth and respond to speed and blossom updates
    fn execute_stage(&mut self, dual_module: &DualModuleRTL, instruction: &Instruction);
    /// update the node according to the updated speed and length after growth
    fn update_stage(&mut self, dual_module: &DualModuleRTL, instruction: &Instruction);
    /// generate a response after the update stage (and optionally, write back to memory)
    fn write_stage(&self, dual_module: &DualModuleRTL, instruction: &Instruction) -> Option<Response>;
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub speed: DualNodeGrowState,
    pub grown: Weight,
    pub is_virtual: bool,
    pub is_defect: bool,
    pub node_index: Option<NodeIndex>, // propagated_dual_node
    pub root_index: Option<NodeIndex>, // propagated_grandson_dual_node
    /// shadow index is usually equal to the node/root_index, but only updated when `grown==0` and `speed==Shrink`;
    /// when this happens, it picks any peer who is growing; this allows the conflict to be detected across a zero node
    pub shadow_node_index: Option<NodeIndex>,
    pub shadow_root_index: Option<NodeIndex>,
    pub shadow_speed: DualNodeGrowState,
}

impl Vertex {
    pub fn get_speed(&self) -> Weight {
        match self.speed {
            DualNodeGrowState::Stay => 0,
            DualNodeGrowState::Shrink => -1,
            DualNodeGrowState::Grow => 1,
        }
    }

    pub fn get_shadow_speed(&self) -> Weight {
        match self.shadow_speed {
            DualNodeGrowState::Stay => 0,
            DualNodeGrowState::Shrink => -1,
            DualNodeGrowState::Grow => 1,
        }
    }
}

impl DualPipelined for Vertex {
    fn execute_stage(&mut self, dual_module: &DualModuleRTL, instruction: &Instruction) {
        match instruction {
            Instruction::AddDefectVertex { vertex, node } => {
                if *vertex == self.vertex_index {
                    self.is_defect = true;
                    self.speed = DualNodeGrowState::Grow;
                    self.root_index = Some(*node);
                    self.node_index = Some(*node);
                }
            }
            Instruction::SetSpeed { node, speed } => {
                if Some(*node) == self.node_index {
                    self.speed = *speed;
                }
            }
            Instruction::Grow { length } => {
                self.grown += self.get_speed() * length;
                assert!(self.grown >= 0);
            }
            Instruction::SetBlossom { node, blossom } => {
                if Some(*node) == self.node_index || Some(*node) == self.root_index {
                    self.node_index = Some(*blossom);
                    self.speed = DualNodeGrowState::Grow;
                }
            }
            Instruction::FindObstacle { .. } => {
                self.shadow_node_index = self.node_index;
                self.shadow_root_index = self.root_index;
                self.shadow_speed = self.speed;
                if self.speed != DualNodeGrowState::Shrink || self.grown != 0 {
                    return;
                }
                // search for a growing peer
                let growing_edge_index = self.edge_indices.iter().find(|edge_index: &&EdgeIndex| {
                    let edge = &dual_module.edges[**edge_index];
                    let peer_index = edge.get_peer(self.vertex_index);
                    let peer = &dual_module.vertices[peer_index];
                    peer.get_speed() > 0 && edge.left_growth + edge.right_growth == edge.weight
                });
                if growing_edge_index.is_none() {
                    return; // in reality this won't happen
                }
                let edge_index = *growing_edge_index.unwrap();
                let edge = &dual_module.edges[edge_index];
                let peer_index = edge.get_peer(self.vertex_index);
                let peer_vertex = &dual_module.vertices[peer_index];
                self.shadow_node_index = peer_vertex.node_index;
                self.shadow_root_index = peer_vertex.root_index;
                self.shadow_speed = DualNodeGrowState::Grow;
            }
        }
    }

    fn update_stage(&mut self, dual_module: &DualModuleRTL, _instruction: &Instruction) {
        // is there any growing peer trying to propagate to this node?
        let propagating_peer: Option<&Vertex> = {
            // find a peer node with positive growth and fully-grown edge
            self.edge_indices
                .iter()
                .map(|&edge_index| {
                    let edge = &dual_module.edges[edge_index];
                    let peer_index = edge.get_peer(self.vertex_index);
                    let peer = &dual_module.vertices[peer_index];
                    if edge.is_tight_from(peer_index) && peer.speed == DualNodeGrowState::Grow {
                        Some(peer)
                    } else {
                        None
                    }
                })
                .reduce(|a, b| a.or(b))
                .unwrap()
        };
        // is this node contributing to at least one
        if !self.is_defect && !self.is_virtual && self.grown == 0 {
            if let Some(peer) = propagating_peer {
                self.node_index = peer.node_index;
                self.root_index = peer.root_index;
                self.speed = peer.speed;
            } else {
                self.node_index = None;
                self.root_index = None;
                self.speed = DualNodeGrowState::Stay;
            }
        }
    }

    // generate a response
    fn write_stage(&self, _dual_module: &DualModuleRTL, _instruction: &Instruction) -> Option<Response> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub edge_index: EdgeIndex,
    pub weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    pub left_growth: Weight,
    pub right_growth: Weight,
    /// information that is passed to neighboring vertex
    // pub 
}

impl Edge {
    pub fn is_tight(&self) -> bool {
        self.left_growth + self.right_growth >= self.weight
    }

    pub fn get_peer(&self, vertex: VertexIndex) -> VertexIndex {
        if vertex == self.left_index {
            self.right_index
        } else if vertex == self.right_index {
            self.left_index
        } else {
            panic!("vertex is not incident to the edge, cannot get peer")
        }
    }

    pub fn is_tight_from(&self, vertex: VertexIndex) -> bool {
        if vertex == self.left_index {
            self.left_growth == self.weight
        } else if vertex == self.right_index {
            self.right_growth == self.weight
        } else {
            panic!("invalid input: vertex is not incident to the edge")
        }
    }
}

impl DualPipelined for Edge {
    // compute the next register values
    #[allow(clippy::single_match)]
    fn execute_stage(&mut self, dual_module: &DualModuleRTL, instruction: &Instruction) {
        match instruction {
            Instruction::Grow { length } => {
                let left_vertex = &dual_module.vertices[self.left_index];
                let right_vertex = &dual_module.vertices[self.right_index];
                if left_vertex.node_index != right_vertex.node_index {
                    self.left_growth += left_vertex.get_speed() * length;
                    self.right_growth += right_vertex.get_speed() * length;
                    assert!(self.left_growth >= 0);
                    assert!(self.right_growth >= 0);
                    assert!(self.left_growth + self.right_growth <= self.weight);
                }
            }
            _ => {}
        }
    }

    fn update_stage(&mut self, _dual_module: &DualModuleRTL, _instruction: &Instruction) {}

    // generate a response
    #[allow(clippy::comparison_chain)]
    fn write_stage(&self, dual_module: &DualModuleRTL, instruction: &Instruction) -> Option<Response> {
        if !matches!(instruction, Instruction::FindObstacle { .. }) {
            return None;
        }
        let left_vertex = &dual_module.vertices[self.left_index];
        let right_vertex = &dual_module.vertices[self.right_index];
        if left_vertex.shadow_node_index == right_vertex.shadow_node_index {
            return None;
        }
        let mut max_growth = Weight::MAX;
        let left_speed = left_vertex.get_shadow_speed();
        if left_speed < 0 {
            // normally self.left_growth > 0, unless the defect vertex has yS=0, which suggests two conflicting nodes
            max_growth = std::cmp::min(max_growth, self.left_growth);
        } else if left_speed > 0 {
            max_growth = std::cmp::min(max_growth, self.weight - self.left_growth);
        }
        let right_speed = right_vertex.get_shadow_speed();
        if right_speed < 0 {
            // normally self.left_growth > 0, unless the defect vertex has yS=0, which suggests two conflicting nodes
            max_growth = std::cmp::min(max_growth, self.right_growth);
        } else if right_speed > 0 {
            max_growth = std::cmp::min(max_growth, self.weight - self.right_growth);
        }
        let joint_speed = left_speed + right_speed;
        if joint_speed > 0 {
            let remaining = self.weight - self.left_growth - self.right_growth;
            if remaining == 0 {
                return Some(Response::Conflict {
                    node_1: left_vertex.shadow_node_index.unwrap(),
                    touch_1: left_vertex.shadow_root_index.unwrap(),
                    vertex_1: self.left_index,
                    node_2: right_vertex.shadow_node_index.unwrap(),
                    touch_2: right_vertex.shadow_root_index.unwrap(),
                    vertex_2: self.right_index,
                });
            }
            max_growth = std::cmp::min(max_growth, remaining / joint_speed);
        }
        if max_growth == 0 {}
        Some(Response::NonZeroGrow { length: max_growth })
    }
}

impl FusionVisualizer for DualModuleRTL {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let vertices: Vec<serde_json::Value> = self
            .vertices
            .iter()
            .map(|vertex| {
                let mut value = json!({
                    if abbrev { "v" } else { "is_virtual" }: i32::from(vertex.is_virtual),
                    if abbrev { "s" } else { "is_defect" }: i32::from(vertex.is_defect),
                });
                if let Some(node_index) = vertex.node_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "p" } else { "propagated_dual_node" }).to_string(),
                        json!(node_index),
                    );
                }
                if let Some(root_index) = vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "pg" } else { "propagated_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                value
            })
            .collect();
        let edges: Vec<serde_json::Value> = self
            .edges
            .iter()
            .map(|edge| {
                let mut value = json!({
                    if abbrev { "w" } else { "weight" }: edge.weight,
                    if abbrev { "l" } else { "left" }: edge.left_index,
                    if abbrev { "r" } else { "right" }: edge.right_index,
                    if abbrev { "lg" } else { "left_growth" }: edge.left_growth,
                    if abbrev { "rg" } else { "right_growth" }: edge.right_growth,
                });
                let left_vertex = &self.vertices[edge.left_index];
                let right_vertex = &self.vertices[edge.right_index];
                if let Some(node_index) = left_vertex.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "ld" } else { "left_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = left_vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "lgd" } else { "left_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                if let Some(node_index) = right_vertex.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "rd" } else { "right_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = right_vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "rgd" } else { "right_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                value
            })
            .collect();
        json!({
            "vertices": vertices,
            "edges": edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusion_blossom::example_codes::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_rtl_basic_1() {
        // cargo test dual_module_rtl_basic_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_basic_1.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleRTL::new_empty(&initializer);
        // a simple syndrome
        code.vertices[19].is_defect = true;
        code.vertices[25].is_defect = true;
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
        let dual_node_25_ptr = interface_ptr.read_recursive().nodes[1].clone().unwrap();
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("grow to 0.5".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("grow to 1".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("grow to 1.5".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // set to shrink
        dual_module.set_grow_state(&dual_node_19_ptr, DualNodeGrowState::Shrink);
        dual_module.set_grow_state(&dual_node_25_ptr, DualNodeGrowState::Shrink);
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("shrink to 1".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("shrink to 0.5".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        dual_module.grow(half_weight);
        visualizer
            .snapshot_combined("shrink to 0".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }

    #[test]
    fn dual_module_rtl_blossom_basics() {
        // cargo test dual_module_rtl_blossom_basics -- --nocapture
        let visualize_filename = "dual_module_rtl_blossom_basics.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleRTL::new_empty(&initializer);
        // try to work on a simple syndrome
        code.vertices[19].is_defect = true;
        code.vertices[26].is_defect = true;
        code.vertices[35].is_defect = true;
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
        let dual_node_26_ptr = interface_ptr.read_recursive().nodes[1].clone().unwrap();
        let dual_node_35_ptr = interface_ptr.read_recursive().nodes[2].clone().unwrap();
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 6 * half_weight);
        visualizer
            .snapshot_combined("before create blossom".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        let nodes_circle = vec![dual_node_19_ptr.clone(), dual_node_26_ptr.clone(), dual_node_35_ptr.clone()];
        interface_ptr.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let dual_node_blossom = interface_ptr.create_blossom(nodes_circle, vec![], &mut dual_module);
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 7 * half_weight);
        visualizer
            .snapshot_combined("blossom grow half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 8 * half_weight);
        visualizer
            .snapshot_combined("blossom grow half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 9 * half_weight);
        visualizer
            .snapshot_combined("blossom grow half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        interface_ptr.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 8 * half_weight);
        visualizer
            .snapshot_combined("blossom shrink half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 6 * half_weight);
        visualizer
            .snapshot_combined("blossom shrink weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        interface_ptr.expand_blossom(dual_node_blossom, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_19_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_35_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 3 * half_weight);
        visualizer
            .snapshot_combined(
                "individual shrink half weight".to_string(),
                vec![&interface_ptr, &dual_module],
            )
            .unwrap();
    }

    #[test]
    fn dual_module_rtl_stop_reason_1() {
        // cargo test dual_module_rtl_stop_reason_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_stop_reason_1.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleRTL::new_empty(&initializer);
        // try to work on a simple syndrome
        code.vertices[19].is_defect = true;
        code.vertices[25].is_defect = true;
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
        let dual_node_25_ptr = interface_ptr.read_recursive().nodes[1].clone().unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 4 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 6 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length
                .peek()
                .unwrap()
                .is_conflicting(&dual_node_19_ptr, &dual_node_25_ptr),
            "unexpected: {:?}",
            group_max_update_length
        );
    }

    #[test]
    fn dual_module_rtl_stop_reason_2() {
        // cargo test dual_module_rtl_stop_reason_2 -- --nocapture
        let visualize_filename = "dual_module_rtl_stop_reason_2.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleRTL::new_empty(&initializer);
        // try to work on a simple syndrome
        code.vertices[18].is_defect = true;
        code.vertices[26].is_defect = true;
        code.vertices[34].is_defect = true;
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // create dual nodes and grow them by half length
        let dual_node_18_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
        let dual_node_26_ptr = interface_ptr.read_recursive().nodes[1].clone().unwrap();
        let dual_node_34_ptr = interface_ptr.read_recursive().nodes[2].clone().unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 3 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length
                .peek()
                .unwrap()
                .is_conflicting(&dual_node_18_ptr, &dual_node_26_ptr)
                || group_max_update_length
                    .peek()
                    .unwrap()
                    .is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr),
            "unexpected: {:?}",
            group_max_update_length
        );
        // first match 18 and 26
        interface_ptr.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Stay, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Stay, &mut dual_module);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length
                .peek()
                .unwrap()
                .is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr),
            "unexpected: {:?}",
            group_max_update_length
        );
        // 34 touches 26, so it will grow the tree by absorbing 18 and 26
        interface_ptr.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 4 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length
                .peek()
                .unwrap()
                .is_conflicting(&dual_node_18_ptr, &dual_node_34_ptr),
            "unexpected: {:?}",
            group_max_update_length
        );
        // for a blossom because 18 and 34 come from the same alternating tree
        let dual_node_blossom = interface_ptr.create_blossom(
            vec![dual_node_18_ptr.clone(), dual_node_26_ptr.clone(), dual_node_34_ptr.clone()],
            vec![],
            &mut dual_module,
        );
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 6 * half_weight);
        visualizer
            .snapshot_combined("grow blossom".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 8 * half_weight);
        visualizer
            .snapshot_combined("grow blossom".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length.peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 23))
                || group_max_update_length.peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 39)),
            "unexpected: {:?}",
            group_max_update_length
        );
        // blossom touches virtual boundary, so it's matched
        interface_ptr.set_grow_state(&dual_node_blossom, DualNodeGrowState::Stay, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length.is_empty(),
            "unexpected: {:?}",
            group_max_update_length
        );
        // also test the reverse procedure: shrinking and expanding blossom
        interface_ptr.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 6 * half_weight);
        visualizer
            .snapshot_combined("shrink blossom".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // before expand
        interface_ptr.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 4 * half_weight);
        visualizer
            .snapshot_combined("shrink blossom".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // cannot shrink anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(
            group_max_update_length.peek().unwrap() == &MaxUpdateLength::BlossomNeedExpand(dual_node_blossom.clone()),
            "unexpected: {:?}",
            group_max_update_length
        );
        // expand blossom
        interface_ptr.expand_blossom(dual_node_blossom, &mut dual_module);
        // regain access to underlying nodes
        interface_ptr.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface_ptr.set_grow_state(&dual_node_34_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(
            group_max_update_length.get_none_zero_growth(),
            Some(2 * half_weight),
            "unexpected: {:?}",
            group_max_update_length
        );
        interface_ptr.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface_ptr.sum_dual_variables(), 2 * half_weight);
        visualizer
            .snapshot_combined("shrink".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }

    // phenomena: index out of bound error
    // command: cargo run --release -- benchmark 7 0.15 --code-type code-capacity-repetition-code --pb-message 'repetition 7 0.1' --total-rounds 1000000 --verifier fusion-serial --use-deterministic-seed --starting-iteration 472872 --print-syndrome-pattern
    // bug: forgot to clear the blossom tracker

    // phenomena: infinite loop
    // command: cargo run --release -- benchmark 7 0.15 --code-type code-capacity-planar-code --total-rounds 1000000 --verifier fusion-serial --use-deterministic-seed --starting-iteration 7 --print-syndrome-pattern
    // bug: constantly reporting "Conflicting((22, 0), (22, 5))" because the vertex forgot to check whether the node is the same

    // phenomena: unexpected perfect matching weight 6000 vs 5000
    // command: cargo run --release -- benchmark 7 0.03 --code-type code-capacity-planar-code --total-rounds 1000000 --verifier fusion-serial --use-deterministic-seed --starting-iteration 820 --enable-visualizer
    // bug: the conflict reported is wrong: should be TouchingVirtual but reported a wrong Conflicting;
    // cause: when checking for conflicts at a real node, I forgot to add condition that only checks fully-grown edges
}
