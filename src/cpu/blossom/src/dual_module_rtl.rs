//! Register Transfer Level (RTL) Dual Module
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use crate::dual_module_adaptor::*;
use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde_json::json;

#[derive(Debug)]
pub struct DualModuleRTLDriver {
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub maximum_growth: Weight,
    // pre-matching optimization that doesn't report qualified local matchings
    pub use_pre_matching: bool,
}

pub type DualModuleRTL = DualModuleStackless<DualDriverTracked<DualModuleRTLDriver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleRTL {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        DualModuleStackless::new(DualDriverTracked::new(DualModuleRTLDriver::new_empty(initializer)))
    }
}

pub type DualModuleRTLAdaptor = DualModuleAdaptor<DualModuleRTL>;

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
        let Response::NonZeroGrow { length: length1 } = resp1 else {
            unreachable!()
        };
        let Response::NonZeroGrow { length: length2 } = resp2 else {
            unreachable!()
        };
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

impl DualModuleRTLDriver {
    pub fn new_empty(initializer: &SolverInitializer) -> Self {
        let mut behavior = Self {
            initializer: initializer.clone(),
            vertices: vec![],
            edges: vec![],
            maximum_growth: Weight::MAX,
            use_pre_matching: false,
        };
        behavior.clear();
        behavior
    }

    pub fn clear(&mut self) {
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
                permit_pre_matching: false,
                do_pre_matching: false,
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
                is_tight: false,
                do_pre_matching: false,
            });
            for vertex_index in [i, j] {
                self.vertices[vertex_index].edge_indices.push(edge_index);
            }
        }
        self.maximum_growth = Weight::MAX;
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> Option<Response> {
        if self.use_pre_matching {
            pipeline_staged!(self, instruction, pre_count_stage);
            pipeline_staged!(self, instruction, pre_match_stage);
        }
        pipeline_staged!(self, instruction, execute_stage);
        pipeline_staged!(self, instruction, update_stage);
        let response = self
            .vertices
            .iter()
            .map(|vertex| vertex.write_stage(self, &instruction))
            .chain(self.edges.iter().map(|edge| edge.write_stage(self, &instruction)))
            .reduce(Response::reduce)
            .unwrap();
        response
    }

    /// get all the edges that are pre-matched in the graph
    pub fn get_pre_matchings(&self) -> Vec<EdgeIndex> {
        if !self.use_pre_matching {
            return vec![];
        }
        self.edges
            .iter()
            .filter(|edge| edge.do_pre_matching)
            .map(|edge| edge.edge_index)
            .collect()
    }

    pub fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

impl DualStacklessDriver for DualModuleRTLDriver {
    fn reset(&mut self) {
        self.clear();
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.execute_instruction(Instruction::SetSpeed {
            node: node.get() as NodeIndex,
            speed: match speed {
                CompactGrowState::Stay => DualNodeGrowState::Stay,
                CompactGrowState::Grow => DualNodeGrowState::Grow,
                CompactGrowState::Shrink => DualNodeGrowState::Shrink,
            },
        });
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.execute_instruction(Instruction::SetBlossom {
            node: node.get() as NodeIndex,
            blossom: blossom.get() as NodeIndex,
        });
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        let mut grown: Weight = 0;
        loop {
            let return_value = self
                .execute_instruction(Instruction::FindObstacle { region_preference: 0 })
                .unwrap();
            match return_value {
                Response::NonZeroGrow { length } => {
                    if length == Weight::MAX {
                        return (CompactObstacle::None, grown as CompactWeight);
                    } else {
                        let length = std::cmp::min(length, self.maximum_growth);
                        if length == 0 {
                            return (CompactObstacle::GrowLength { length: 0 }, grown as CompactWeight);
                        } else {
                            self.execute_instruction(Instruction::Grow { length });
                            self.maximum_growth -= length;
                            grown += length;
                        }
                    }
                }
                Response::Conflict {
                    node_1,
                    node_2,
                    touch_1,
                    touch_2,
                    vertex_1,
                    vertex_2,
                } => {
                    let (node_1, node_2, touch_1, touch_2, vertex_1, vertex_2) = if node_2 == VIRTUAL_NODE_INDEX {
                        (node_1, node_2, touch_1, touch_2, vertex_1, vertex_2)
                    } else {
                        (node_2, node_1, touch_2, touch_1, vertex_2, vertex_1)
                    };
                    return (
                        CompactObstacle::Conflict {
                            node_1: ni!(node_1).option(),
                            node_2: if node_2 != VIRTUAL_NODE_INDEX {
                                ni!(node_2).option()
                            } else {
                                None.into()
                            },
                            touch_1: ni!(touch_1).option(),
                            touch_2: if touch_2 != VIRTUAL_NODE_INDEX {
                                ni!(touch_2).option()
                            } else {
                                None.into()
                            },
                            vertex_1: ni!(vertex_1),
                            vertex_2: ni!(vertex_2),
                        },
                        grown as CompactWeight,
                    );
                }
                _ => unreachable!(),
            };
        }
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        self.execute_instruction(Instruction::AddDefectVertex {
            vertex: vertex.get() as VertexIndex,
            node: node.get() as NodeIndex,
        });
    }
}

impl DualTrackedDriver for DualModuleRTLDriver {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        self.maximum_growth = maximum_growth as Weight;
        self.find_obstacle()
    }
}

pub trait DualPipelined {
    /// load data from BRAM (optional)
    fn load_stage(&mut self, _dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {}
    /// pre count stage that checks how many tight edges are surrounding each vertex
    fn pre_count_stage(&mut self, _dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {}
    /// pre matching stage marks the vertices as pre-matched if one of its neighbor edges is marked pre-matching
    fn pre_match_stage(&mut self, _dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {}
    /// execute growth and respond to speed and blossom updates
    fn execute_stage(&mut self, dual_module: &DualModuleRTLDriver, instruction: &Instruction);
    /// update the node according to the updated speed and length after growth
    fn update_stage(&mut self, dual_module: &DualModuleRTLDriver, instruction: &Instruction);
    /// generate a response after the update stage (and optionally, write back to memory)
    fn write_stage(&self, dual_module: &DualModuleRTLDriver, instruction: &Instruction) -> Option<Response>;
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
    /// when a vertex is only surrounded by a single vertex, it reports to an edge;
    /// if both vertices of an edge is surrounded by a single tight edge, then this edge can be locally matched
    pub permit_pre_matching: bool,
    pub do_pre_matching: bool,
}

impl Vertex {
    pub fn get_speed(&self) -> Weight {
        if self.do_pre_matching {
            0
        } else {
            match self.speed {
                DualNodeGrowState::Stay => 0,
                DualNodeGrowState::Shrink => -1,
                DualNodeGrowState::Grow => 1,
            }
        }
    }

    pub fn get_shadow_speed(&self) -> Weight {
        if self.do_pre_matching {
            0
        } else {
            match self.shadow_speed {
                DualNodeGrowState::Stay => 0,
                DualNodeGrowState::Shrink => -1,
                DualNodeGrowState::Grow => 1,
            }
        }
    }

    pub fn get_updated_grown(&self, instruction: &Instruction) -> Weight {
        match instruction {
            Instruction::Grow { length } => self.grown + self.get_speed() * length,
            _ => self.grown,
        }
    }
}

impl DualPipelined for Vertex {
    fn pre_count_stage(&mut self, dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {
        self.permit_pre_matching = self.speed == DualNodeGrowState::Grow
            && self
                .edge_indices
                .iter()
                .filter(|&&edge_index| {
                    let edge = &dual_module.edges[edge_index];
                    let peer_index = edge.get_peer(self.vertex_index);
                    let peer = &dual_module.vertices[peer_index];
                    self.grown + peer.grown >= edge.weight
                })
                .count()
                == 1;
    }

    fn pre_match_stage(&mut self, dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {
        self.do_pre_matching = self.edge_indices.iter().any(|&edge_index| {
            let edge = &dual_module.edges[edge_index];
            edge.get_do_pre_matching(dual_module)
        });
    }

    fn execute_stage(&mut self, _dual_module: &DualModuleRTLDriver, instruction: &Instruction) {
        match instruction {
            Instruction::SetSpeed { node, speed } => {
                if Some(*node) == self.node_index {
                    self.speed = *speed;
                }
            }
            Instruction::SetBlossom { node, blossom } => {
                if Some(*node) == self.node_index || Some(*node) == self.root_index {
                    self.node_index = Some(*blossom);
                    self.speed = DualNodeGrowState::Grow;
                }
            }
            Instruction::Grow { .. } => {
                self.grown = self.get_updated_grown(instruction);
                assert!(
                    self.grown >= 0,
                    "vertex {} has negative grown value {}",
                    self.vertex_index,
                    self.grown
                );
            }
            Instruction::AddDefectVertex { vertex, node } => {
                if *vertex == self.vertex_index {
                    self.is_defect = true;
                    self.speed = DualNodeGrowState::Grow;
                    self.root_index = Some(*node);
                    self.node_index = Some(*node);
                }
            }
            _ => {}
        }
    }

    fn update_stage(&mut self, dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {
        // is there any growing peer trying to propagate to this node?
        let propagating_peer: Option<&Vertex> = if self.grown == 0 && !self.edge_indices.is_empty() {
            // find a peer node with positive growth and fully-grown edge
            self.edge_indices
                .iter()
                .map(|&edge_index| {
                    let edge = &dual_module.edges[edge_index];
                    let peer_index = edge.get_peer(self.vertex_index);
                    let peer = &dual_module.vertices[peer_index];
                    if edge.is_tight && peer.speed == DualNodeGrowState::Grow {
                        Some(peer)
                    } else {
                        None
                    }
                })
                .reduce(|a, b| a.or(b))
                .unwrap()
        } else {
            None
        };
        self.shadow_node_index = self.node_index;
        self.shadow_root_index = self.root_index;
        self.shadow_speed = self.speed;
        if self.speed == DualNodeGrowState::Shrink && self.grown == 0 {
            if let Some(peer) = propagating_peer {
                self.shadow_node_index = peer.node_index;
                self.shadow_root_index = peer.root_index;
                self.shadow_speed = DualNodeGrowState::Grow;
            }
        }
        if !self.is_defect && !self.is_virtual && self.grown == 0 {
            if let Some(peer) = propagating_peer {
                self.node_index = peer.node_index;
                self.root_index = peer.root_index;
                self.speed = DualNodeGrowState::Grow;
            } else {
                self.node_index = None;
                self.root_index = None;
                self.speed = DualNodeGrowState::Stay;
            }
        }
    }

    // generate a response
    fn write_stage(&self, _dual_module: &DualModuleRTLDriver, _instruction: &Instruction) -> Option<Response> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub edge_index: EdgeIndex,
    pub weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    /// information passing to neighboring vertex
    pub is_tight: bool,
    pub do_pre_matching: bool,
}

impl Edge {
    pub fn get_peer(&self, vertex: VertexIndex) -> VertexIndex {
        if vertex == self.left_index {
            self.right_index
        } else if vertex == self.right_index {
            self.left_index
        } else {
            panic!("vertex is not incident to the edge, cannot get peer")
        }
    }

    pub fn get_do_pre_matching(&self, dual_module: &DualModuleRTLDriver) -> bool {
        let left_vertex = &dual_module.vertices[self.left_index];
        let right_vertex = &dual_module.vertices[self.right_index];
        left_vertex.permit_pre_matching
            && right_vertex.permit_pre_matching
            && (left_vertex.grown + right_vertex.grown >= self.weight)
            && (left_vertex.node_index != right_vertex.node_index)
    }
}

impl DualPipelined for Edge {
    fn pre_match_stage(&mut self, dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {
        self.do_pre_matching = self.get_do_pre_matching(dual_module);
    }

    // compute the next register values
    #[allow(clippy::single_match)]
    fn execute_stage(&mut self, dual_module: &DualModuleRTLDriver, instruction: &Instruction) {
        let left_vertex = &dual_module.vertices[self.left_index];
        let right_vertex = &dual_module.vertices[self.right_index];
        self.is_tight =
            left_vertex.get_updated_grown(instruction) + right_vertex.get_updated_grown(instruction) >= self.weight;
    }

    fn update_stage(&mut self, _dual_module: &DualModuleRTLDriver, _instruction: &Instruction) {}

    // generate a response
    #[allow(clippy::comparison_chain)]
    fn write_stage(&self, dual_module: &DualModuleRTLDriver, instruction: &Instruction) -> Option<Response> {
        if !matches!(instruction, Instruction::FindObstacle { .. }) {
            return None;
        }
        let left_vertex = &dual_module.vertices[self.left_index];
        let right_vertex = &dual_module.vertices[self.right_index];
        if left_vertex.shadow_node_index == right_vertex.shadow_node_index {
            return Some(Response::NonZeroGrow { length: Weight::MAX });
        }
        let mut max_growth = Weight::MAX;
        let left_speed = left_vertex.get_shadow_speed();
        if left_speed < 0 {
            // normally self.left_growth > 0, unless the defect vertex has yS=0, which suggests two conflicting nodes
            max_growth = std::cmp::min(max_growth, left_vertex.grown);
        } else if left_speed > 0 {
            // update 2023.11.13: we don't really need to check this, because it will be bounded by other conditions.
            // max_growth = std::cmp::min(max_growth, self.weight - left_vertex.grown);
        }
        let right_speed = right_vertex.get_shadow_speed();
        if right_speed < 0 {
            // normally self.left_growth > 0, unless the defect vertex has yS=0, which suggests two conflicting nodes
            max_growth = std::cmp::min(max_growth, right_vertex.grown);
        } else if right_speed > 0 {
            // update 2023.11.13: we don't really need to check this, because it will be bounded by other conditions.
            // max_growth = std::cmp::min(max_growth, self.weight - right_vertex.grown);
        }
        let joint_speed = left_speed + right_speed;
        if joint_speed > 0 {
            let remaining = self.weight - left_vertex.grown - right_vertex.grown;
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
            assert!(
                remaining % joint_speed == 0,
                "found a case where the reported maxGrowth is rounding down, edge {}",
                self.edge_index
            );
            max_growth = std::cmp::min(max_growth, remaining / joint_speed);
        }
        Some(Response::NonZeroGrow { length: max_growth })
    }
}

impl FusionVisualizer for DualModuleRTLAdaptor {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        self.dual_module.driver.driver.snapshot(abbrev)
    }
}

impl FusionVisualizer for DualModuleRTLDriver {
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
                let left_vertex = &self.vertices[edge.left_index];
                let right_vertex = &self.vertices[edge.right_index];
                let mut value = json!({
                    if abbrev { "w" } else { "weight" }: edge.weight,
                    if abbrev { "l" } else { "left" }: edge.left_index,
                    if abbrev { "r" } else { "right" }: edge.right_index,
                    if abbrev { "lg" } else { "left_growth" }: left_vertex.grown,
                    if abbrev { "rg" } else { "right_growth" }: right_vertex.grown,
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
pub mod tests {
    use super::*;
    use crate::mwpm_solver::*;
    use fusion_blossom::example_codes::*;
    use fusion_blossom::mwpm_solver::*;
    use fusion_blossom::primal_module::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_rtl_adaptor_basic_1() {
        // cargo test dual_module_rtl_adaptor_basic_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_adaptor_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_rtl_adaptor_basic_standard_syndrome(7, visualize_filename, defect_vertices);
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

    // test a simple blossom
    #[test]
    fn dual_module_rtl_embedded_basic_1() {
        // cargo test dual_module_rtl_embedded_basic_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// test a free node conflict with a virtual boundary
    #[test]
    fn dual_module_rtl_embedded_basic_2() {
        // cargo test dual_module_rtl_embedded_basic_2 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_basic_2.json".to_string();
        let defect_vertices = vec![16];
        dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// test a free node conflict with a matched node (with virtual boundary)
    #[test]
    fn dual_module_rtl_embedded_basic_3() {
        // cargo test dual_module_rtl_embedded_basic_3 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// infinite loop reporting `obstacle: BlossomNeedExpand { blossom: 3000 }`
    #[test]
    fn dual_module_rtl_embedded_debug_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_rtl_embedded_debug_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_debug_1.json".to_string();
        let defect_vertices = vec![1, 9, 17, 19, 33];
        dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// infinite loop
    /// the reason is, when absorbing multiple blossoms and defects into a single blossom,
    /// the dual module doesn't need to update the speed, but the blossom tracker needs to know
    /// what are the absorbed blossoms; otherwise it is going to assume that a child blossom is still
    /// shrinking, despite that it is essentially at the `Stay` state implicitly.
    #[test]
    fn dual_module_rtl_embedded_debug_2() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_rtl_embedded_debug_2 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_debug_2.json".to_string();
        let defect_vertices = vec![64, 66, 67, 77, 79, 90, 91, 99, 100, 101, 102, 112, 125];
        dual_module_rtl_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices);
    }

    /// assertion failed: self.grown >= 0', src/dual_module_rtl.rs:527:17
    /// the reason is the blossom tracker is not informed some incremental grown values and thus
    /// call `set_maximum_growth` with wrong parameters
    #[test]
    fn dual_module_rtl_embedded_debug_3() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_rtl_embedded_debug_3 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_debug_3.json".to_string();
        let defect_vertices = vec![29, 69, 88, 89, 91, 94, 108, 110, 111, 112, 113, 130, 131, 132, 150];
        dual_module_rtl_embedded_basic_standard_syndrome(19, visualize_filename, defect_vertices);
        // dual_module_rtl_adaptor_basic_standard_syndrome(19, visualize_filename, defect_vertices);
    }

    /// evaluate a new feature of pre matching without compromises global optimal result
    #[test]
    fn dual_module_rtl_embedded_pre_matching_basic_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_rtl_embedded_pre_matching_basic_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_pre_matching_basic_1.json".to_string();
        let defect_vertices = vec![13, 14];
        dual_module_rtl_pre_matching_standard_syndrome(5, visualize_filename, defect_vertices);
    }

    /// bug: the growth of a pre-matched vertex should be stopped, but it's not
    #[test]
    fn dual_module_rtl_embedded_pre_matching_debug_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_rtl_embedded_pre_matching_debug_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_embedded_pre_matching_debug_1.json".to_string();
        let defect_vertices = vec![0, 4, 9];
        dual_module_rtl_pre_matching_standard_syndrome(3, visualize_filename, defect_vertices);
        // dual_module_rtl_adaptor_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    pub fn dual_module_rtl_embedded_basic_standard_syndrome_optional_viz<Solver: PrimalDualSolver + Sized>(
        d: VertexNum,
        visualize_filename: Option<String>,
        defect_vertices: Vec<VertexIndex>,
        constructor: impl FnOnce(&SolverInitializer, &Vec<VisualizePosition>) -> Box<Solver>,
    ) -> Box<Solver> {
        println!("{defect_vertices:?}");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(d, 0.1, half_weight);
        let mut visualizer = match visualize_filename.as_ref() {
            Some(visualize_filename) => {
                let visualizer = Visualizer::new(
                    option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
                    code.get_positions(),
                    true,
                )
                .unwrap();
                print_visualize_link(visualize_filename.clone());
                Some(visualizer)
            }
            None => None,
        };
        // create dual module
        let initializer = code.get_initializer();
        code.set_defect_vertices(&defect_vertices);
        let syndrome = code.get_syndrome();
        let mut solver = stacker::grow(crate::util::MAX_NODE_NUM * 1024, || -> Box<Solver> {
            constructor(&initializer, &code.get_positions())
        });
        solver.solve_visualizer(&syndrome, visualizer.as_mut());
        let subgraph = solver.subgraph_visualizer(visualizer.as_mut());
        let mut standard_solver = SolverSerial::new(&initializer);
        standard_solver.solve_visualizer(&syndrome, None);
        let standard_subgraph = standard_solver.subgraph_visualizer(None);
        let mut subgraph_builder = SubGraphBuilder::new(&initializer);
        subgraph_builder.load_subgraph(&subgraph);
        let total_weight = subgraph_builder.total_weight();
        subgraph_builder.load_subgraph(&standard_subgraph);
        let standard_total_weight = subgraph_builder.total_weight();
        assert_eq!(total_weight, standard_total_weight);
        solver
    }

    pub fn dual_module_rtl_embedded_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> Box<SolverEmbeddedRTL> {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename),
            defect_vertices,
            |initializer, _| Box::new(SolverEmbeddedRTL::new(initializer)),
        )
    }

    pub fn dual_module_rtl_pre_matching_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> Box<SolverEmbeddedRTL> {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename),
            defect_vertices,
            |initializer, _| {
                let mut solver = SolverEmbeddedRTL::new(initializer);
                solver.dual_module.driver.driver.use_pre_matching = true;
                Box::new(solver)
            },
        )
    }

    pub fn dual_module_rtl_adaptor_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> Box<SolverDualRTL> {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename),
            defect_vertices,
            |initializer, _| Box::new(SolverDualRTL::new(initializer)),
        )
    }
}
