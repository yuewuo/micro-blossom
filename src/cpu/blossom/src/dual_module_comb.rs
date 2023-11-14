//! A dual module implemented with combinatorial logic, for easy modeling of the real hardware
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use crate::dual_module_adaptor::*;
use crate::dual_module_comb_edge::*;
use crate::dual_module_comb_vertex::*;
use crate::util::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde_json::json;
use std::collections::BTreeSet;

pub struct DualModuleCombDriver {
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub maximum_growth: CompactWeight,
    // pre-matching optimization that doesn't report qualified local matchings
    pub use_pre_matching: bool,
    // the current instruction for computing the combinatorial logic
    pub(crate) instruction: Instruction,
}

pub type DualModuleComb = DualModuleStackless<DualDriverTracked<DualModuleCombDriver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleComb {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        DualModuleStackless::new(DualDriverTracked::new(DualModuleCombDriver::new_empty(initializer)))
    }
}

impl DualModuleCombDriver {
    pub fn new_empty(initializer: &SolverInitializer) -> Self {
        let virtual_vertices: BTreeSet<VertexIndex> = initializer.virtual_vertices.iter().cloned().collect();
        let mut all_incident_edges: Vec<Vec<EdgeIndex>> = vec![vec![]; initializer.vertex_num];
        for (edge_index, &(i, j, _weight)) in initializer.weighted_edges.iter().enumerate() {
            for vertex_index in [i, j] {
                all_incident_edges[vertex_index].push(edge_index);
            }
        }
        let mut behavior = Self {
            initializer: initializer.clone(),
            vertices: all_incident_edges
                .into_iter()
                .enumerate()
                .map(|(vertex_index, incident_edges)| {
                    let is_virtual = virtual_vertices.contains(&vertex_index);
                    Vertex::new(vertex_index, incident_edges, is_virtual)
                })
                .collect(),
            edges: initializer
                .weighted_edges
                .iter()
                .enumerate()
                .map(|(edge_index, &(i, j, weight))| Edge::new(edge_index, i, j, weight))
                .collect(),
            maximum_growth: CompactWeight::MAX,
            use_pre_matching: false,
            instruction: Instruction::FindObstacle,
        };
        behavior.clear();
        behavior
    }

    pub fn clear(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.clear();
        }
        for edge in self.edges.iter_mut() {
            edge.clear();
        }
    }

    pub fn register_updated(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.register_updated()
        }
        for edge in self.edges.iter_mut() {
            edge.register_updated()
        }
    }

    pub fn propagate_signals(&mut self, instruction: Instruction) {
        self.instruction = instruction;
        self.register_updated();
    }

    pub fn update_registers(&mut self) {
        for vertex_index in 0..self.vertices.len() {
            let registers = self.vertices[vertex_index].get_write_signals(self).clone();
            self.vertices[vertex_index].registers = registers;
        }
        for edge_index in 0..self.edges.len() {
            let registers = self.edges[edge_index].get_write_signals(self).clone();
            self.edges[edge_index].registers = registers;
        }
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> CompactObstacle {
        self.propagate_signals(instruction);
        let response = self
            .vertices
            .iter()
            .map(|vertex| vertex.get_response(self).clone())
            .chain(self.edges.iter().map(|edge| edge.get_response(self).clone()))
            .reduce(CompactObstacle::reduce)
            .unwrap();
        self.update_registers();
        response
    }

    /// get all the edges that are pre-matched in the graph
    pub fn get_pre_matchings(&self) -> Vec<EdgeIndex> {
        if !self.use_pre_matching {
            return vec![];
        }
        self.edges
            .iter()
            .filter(|edge| edge.get_do_pre_matching(self))
            .map(|edge| edge.edge_index)
            .collect()
    }

    pub fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

impl DualStacklessDriver for DualModuleCombDriver {
    fn reset(&mut self) {
        self.clear();
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.execute_instruction(Instruction::SetSpeed {
            node: node.get() as NodeIndex,
            speed,
        });
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.execute_instruction(Instruction::SetBlossom {
            node: node.get() as NodeIndex,
            blossom: blossom.get() as NodeIndex,
        });
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        let mut grown: CompactWeight = 0;
        loop {
            let mut obstacle = self.execute_instruction(Instruction::FindObstacle);
            obstacle.fix_conflict_order();
            match obstacle {
                CompactObstacle::None => unreachable!(),
                CompactObstacle::GrowLength { length } => {
                    if length == CompactWeight::MAX {
                        return (CompactObstacle::None, grown);
                    } else {
                        let length = std::cmp::min(length, self.maximum_growth);
                        if length == 0 {
                            return (CompactObstacle::GrowLength { length: 0 }, grown as CompactWeight);
                        } else {
                            self.execute_instruction(Instruction::Grow {
                                length: length as Weight,
                            });
                            self.maximum_growth -= length;
                            grown += length;
                        }
                    }
                }
                CompactObstacle::Conflict { .. } => return (obstacle, grown),
                _ => unreachable!(),
            }
        }
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        self.execute_instruction(Instruction::AddDefectVertex {
            vertex: vertex.get() as VertexIndex,
            node: node.get() as NodeIndex,
        });
    }
}

impl DualTrackedDriver for DualModuleCombDriver {
    fn set_maximum_growth(&mut self, length: CompactWeight) {
        self.maximum_growth = length;
    }
}

impl FusionVisualizer for DualModuleCombDriver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let vertices: Vec<serde_json::Value> = self
            .vertices
            .iter()
            .map(|vertex| {
                let mut value = json!({
                    if abbrev { "v" } else { "is_virtual" }: i32::from(vertex.registers.is_virtual),
                    if abbrev { "s" } else { "is_defect" }: i32::from(vertex.registers.is_defect),
                });
                if let Some(node_index) = vertex.registers.node_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "p" } else { "propagated_dual_node" }).to_string(),
                        json!(node_index),
                    );
                }
                if let Some(root_index) = vertex.registers.root_index.as_ref() {
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
                    if abbrev { "w" } else { "weight" }: edge.registers.weight,
                    if abbrev { "l" } else { "left" }: edge.left_index,
                    if abbrev { "r" } else { "right" }: edge.right_index,
                    if abbrev { "lg" } else { "left_growth" }: left_vertex.registers.grown,
                    if abbrev { "rg" } else { "right_growth" }: right_vertex.registers.grown,
                });
                let left_vertex = &self.vertices[edge.left_index];
                let right_vertex = &self.vertices[edge.right_index];
                if let Some(node_index) = left_vertex.registers.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "ld" } else { "left_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = left_vertex.registers.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "lgd" } else { "left_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                if let Some(node_index) = right_vertex.registers.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "rd" } else { "right_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = right_vertex.registers.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "rgd" } else { "right_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                value
            })
            .collect();
        let vertices_comb: Vec<serde_json::Value> =
            self.vertices.iter().map(|vertex| vertex.snapshot(abbrev, self)).collect();
        let edges_comb: Vec<serde_json::Value> = self.edges.iter().map(|edge| edge.snapshot(abbrev, self)).collect();
        json!({
            "vertices": vertices,
            "edges": edges,
            "vertices_comb": vertices_comb,
            "edges_comb": edges_comb,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    SetSpeed { node: NodeIndex, speed: CompactGrowState },
    SetBlossom { node: NodeIndex, blossom: NodeIndex },
    AddDefectVertex { vertex: VertexIndex, node: NodeIndex },
    FindObstacle,
    Grow { length: Weight },
}

pub const VIRTUAL_NODE_INDEX: NodeIndex = NodeIndex::MAX;

#[macro_export]
macro_rules! referenced_signal {
    ($signal:expr, $function:expr) => {
        if $signal.borrow().is_some() {
            Ref::map($signal.borrow(), |value| value.as_ref().unwrap())
        } else {
            $signal.borrow_mut().get_or_insert_with($function);
            Ref::map($signal.borrow(), |value| value.as_ref().unwrap())
        }
    };
}
#[allow(unused_imports)]
pub use referenced_signal;

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::dual_module_rtl::tests::*;
    use crate::mwpm_solver::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_comb_basic_1() {
        // cargo test dual_module_comb_basic_1 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// test a free node conflict with a virtual boundary
    #[test]
    fn dual_module_comb_basic_2() {
        // cargo test dual_module_comb_basic_2 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_2.json".to_string();
        let defect_vertices = vec![16];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// test a free node conflict with a matched node (with virtual boundary)
    #[test]
    fn dual_module_comb_basic_3() {
        // cargo test dual_module_comb_basic_3 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// evaluate a new feature of pre matching without compromises global optimal result
    #[test]
    fn dual_module_comb_pre_matching_basic_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_comb_pre_matching_basic_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_basic_1.json".to_string();
        let defect_vertices = vec![13, 14];
        dual_module_comb_pre_matching_standard_syndrome(5, visualize_filename, defect_vertices);
    }

    /// bug: the growth of a pre-matched vertex should be stopped, but it's not
    #[test]
    fn dual_module_comb_pre_matching_debug_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_comb_pre_matching_debug_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_debug_1.json".to_string();
        let defect_vertices = vec![0, 4, 9];
        dual_module_comb_pre_matching_standard_syndrome(3, visualize_filename, defect_vertices);
        // dual_module_rtl_adaptor_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    pub fn dual_module_comb_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverDualComb {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer| SolverDualComb::new(initializer),
        )
    }

    pub fn dual_module_comb_pre_matching_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverDualComb {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer| {
                let mut solver = SolverDualComb::new(initializer);
                solver.dual_module.driver.driver.use_pre_matching = true;
                solver
            },
        )
    }
}
