//! A dual module implemented with combinatorial logic, modeling the real hardware
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use crate::dual_module_adaptor::*;
use crate::dual_module_axi4::DualConfig;
use crate::dual_module_comb_edge::*;
use crate::dual_module_comb_offloading::*;
use crate::dual_module_comb_vertex::*;
use crate::resources::*;
use crate::util::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde::*;
use serde_json::json;
use std::collections::BTreeSet;

pub struct DualModuleCombDriver {
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub offloading_units: Vec<Offloading>,
    pub maximum_growth: CompactWeight,
    /// the current instruction for computing the combinatorial logic
    pub(crate) instruction: Instruction,
    pub config: DualCombConfig,
    pub dual_config: DualConfig,
    pub graph: MicroBlossomSingle,
    /// only enabled when `config.log_instructions` is true
    pub profiler_instruction_history: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualCombConfig {
    /// record instructions into the profile
    #[serde(default = "dual_comb_config_default::log_instructions")]
    pub log_instructions: bool,
}

impl Default for DualCombConfig {
    fn default() -> Self {
        serde_json::from_value(json!({})).unwrap()
    }
}

pub mod dual_comb_config_default {
    pub fn log_instructions() -> bool {
        false
    }
}

pub type DualModuleComb = DualModuleStackless<DualDriverTracked<DualModuleCombDriver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleComb {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        DualModuleStackless::new(DualDriverTracked::new(DualModuleCombDriver::new_empty(initializer)))
    }
}

impl DualModuleCombDriver {
    pub fn new(graph: MicroBlossomSingle, comb_config: DualCombConfig) -> Self {
        let virtual_vertices: BTreeSet<VertexIndex> = graph.virtual_vertices.iter().cloned().collect();
        let mut all_incident_edges: Vec<Vec<EdgeIndex>> = vec![vec![]; graph.vertex_num];
        for (edge_index, &WeightedEdge { l, r, .. }) in graph.weighted_edges.iter().enumerate() {
            for vertex_index in [l, r] {
                all_incident_edges[vertex_index].push(edge_index);
            }
        }
        let initializer = graph.get_initializer();
        let dual_config: DualConfig = Default::default();
        let mut comb_driver = Self {
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
            offloading_units: vec![],
            instruction: Instruction::FindObstacle,
            graph: graph.clone(),
            config: comb_config,
            dual_config: dual_config.clone(),
            profiler_instruction_history: vec![],
        };
        let mut offloading_vec = graph.offloading.0.clone();
        if dual_config.support_layer_fusion {
            let layer_fusion = graph.layer_fusion.as_ref().unwrap();
            for (edge_index, conditioned_vertex) in layer_fusion.fusion_edges.iter() {
                offloading_vec.push(OffloadingType::FusionMatch {
                    edge_index: *edge_index,
                    conditioned_vertex: *conditioned_vertex,
                });
                comb_driver.edges[*edge_index].conditioned_vertex = Some(*conditioned_vertex);
            }
            for (vertex_index, layer_id) in layer_fusion.vertex_layer_id.iter() {
                comb_driver.vertices[*vertex_index].layer_id = Some(*layer_id);
            }
        }
        if dual_config.support_offloading {
            comb_driver.set_offloading_units(&initializer, offloading_vec);
        }
        comb_driver.clear();
        comb_driver
    }

    pub fn set_offloading_units(&mut self, initializer: &SolverInitializer, offloading_types: Vec<OffloadingType>) {
        for vertex in self.vertices.iter_mut() {
            vertex.offloading_indices.clear();
        }
        for edge in self.edges.iter_mut() {
            edge.offloading_indices.clear();
        }
        self.offloading_units = offloading_types
            .into_iter()
            .map(|offloading_type| Offloading::new(offloading_type, initializer))
            .collect();
        // connect the offloading units
        for (offloading_index, offloading) in self.offloading_units.iter().enumerate() {
            for &vertex_index in offloading.affecting_vertices.iter() {
                self.vertices[vertex_index].offloading_indices.push(offloading_index);
            }
            for &edge_index in offloading.affecting_edges.iter() {
                self.edges[edge_index].offloading_indices.push(offloading_index);
            }
        }
    }

    pub fn new_empty(initializer: &SolverInitializer) -> Self {
        let fake_positions = vec![VisualizePosition::new(0., 0., 0.); initializer.vertex_num];
        let config = MicroBlossomSingle::new(initializer, &fake_positions);
        Self::new(config, serde_json::from_value(json!({})).unwrap())
    }

    pub fn clear(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.clear();
        }
        for edge in self.edges.iter_mut() {
            edge.clear();
        }
        for offloading_unit in self.offloading_units.iter_mut() {
            offloading_unit.clear();
        }
    }

    pub fn reset_profiler(&mut self) {
        self.profiler_instruction_history.clear();
    }

    pub fn register_updated(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.register_updated()
        }
        for edge in self.edges.iter_mut() {
            edge.register_updated()
        }
        for offloading_unit in self.offloading_units.iter_mut() {
            offloading_unit.register_updated()
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
        if self.config.log_instructions {
            self.profiler_instruction_history.push(instruction.clone());
        }
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
        self.edges
            .iter()
            .filter(|edge| edge.get_offloading_stalled(self))
            .map(|edge| edge.edge_index)
            .collect()
    }

    pub fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "history": self.profiler_instruction_history,
        })
    }

    /// fuse one layer of defects (in this simulation, the defects are still loaded using `AddDefectVertex`,
    /// but real hardware should be able to load from some channel)
    pub fn fuse_layer(&mut self, layer_id: usize) {
        self.execute_instruction(Instruction::LoadDefectsExternal {
            time: layer_id,
            channel: 0,
        });
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
                    assert!(length >= 0, "report negative grow length");
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
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        self.maximum_growth = maximum_growth;
        self.find_obstacle()
    }
}

impl FusionVisualizer for DualModuleCombDriver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let optional_node_index = |node_index: NodeIndex| {
            if node_index == VIRTUAL_NODE_INDEX {
                None
            } else {
                Some(node_index)
            }
        };
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
                        json!(optional_node_index(*node_index)),
                    );
                }
                if let Some(root_index) = vertex.registers.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "pg" } else { "propagated_grandson_dual_node" }).to_string(),
                        json!(optional_node_index(*root_index)),
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
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "ld" } else { "left_dual_node" }).to_string(),
                        json!(optional_node_index(*node_index)),
                    );
                }
                if let Some(root_index) = left_vertex.registers.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "lgd" } else { "left_grandson_dual_node" }).to_string(),
                        json!(optional_node_index(*root_index)),
                    );
                }
                if let Some(node_index) = right_vertex.registers.node_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "rd" } else { "right_dual_node" }).to_string(),
                        json!(optional_node_index(*node_index)),
                    );
                }
                if let Some(root_index) = right_vertex.registers.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "rgd" } else { "right_grandson_dual_node" }).to_string(),
                        json!(optional_node_index(*root_index)),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    SetSpeed { node: NodeIndex, speed: CompactGrowState },
    SetBlossom { node: NodeIndex, blossom: NodeIndex },
    AddDefectVertex { vertex: VertexIndex, node: NodeIndex },
    FindObstacle,
    Grow { length: Weight },
    LoadDefectsExternal { time: usize, channel: usize },
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
    use crate::ENV_PARAMETER_LOCK;
    use fusion_blossom::example_codes::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_comb_basic_1() {
        // cargo test dual_module_comb_basic_1 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, false);
    }

    /// test a free node conflict with a virtual boundary
    #[test]
    fn dual_module_comb_basic_2() {
        // cargo test dual_module_comb_basic_2 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_2.json".to_string();
        let defect_vertices = vec![16];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, false);
    }

    /// test a free node conflict with a matched node (with virtual boundary)
    #[test]
    fn dual_module_comb_basic_3() {
        // cargo test dual_module_comb_basic_3 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, false);
    }

    /// evaluate a new feature of pre matching without compromises global optimal result
    #[test]
    fn dual_module_comb_pre_matching_basic_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_comb_pre_matching_basic_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_basic_1.json".to_string();
        let defect_vertices = vec![13, 14];
        dual_module_comb_basic_standard_syndrome(5, visualize_filename, defect_vertices, true, false);
    }

    /// bug: the growth of a pre-matched vertex should be stopped, but it's not
    #[test]
    fn dual_module_comb_pre_matching_debug_1() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_comb_pre_matching_debug_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_debug_1.json".to_string();
        let defect_vertices = vec![0, 4, 9];
        dual_module_comb_basic_standard_syndrome(3, visualize_filename, defect_vertices, true, false);
        // dual_module_rtl_adaptor_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    /// bug: 4000 != 5000
    #[test]
    fn dual_module_comb_pre_matching_debug_2() {
        // PRINT_DUAL_CALLS=1 cargo test dual_module_comb_pre_matching_debug_2 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_debug_2.json".to_string();
        let defect_vertices = vec![20, 27, 28, 36, 43, 44, 45, 53];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, true, false);
    }

    // /// evaluate pre-matching with virtual vertex
    #[test]
    fn dual_module_comb_pre_matching_basic_2() {
        // cargo test dual_module_comb_pre_matching_basic_2 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_basic_2.json".to_string();
        let defect_vertices = vec![4];
        let solver = dual_module_comb_basic_standard_syndrome(3, visualize_filename, defect_vertices, true, false);
        assert!(solver.offloaded == 1);
    }

    // /// verify that all single error can be decoded totally offline
    #[test]
    fn dual_module_comb_pre_matching_all_single_error() {
        // cargo test dual_module_comb_pre_matching_all_single_error -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_all_single_error.json".to_string();
        let d = 5;
        let code = CodeCapacityPlanarCode::new(d, 0.1, 500);
        let initializer = code.get_initializer();
        let virtual_vertices: BTreeSet<_> = initializer.virtual_vertices.iter().cloned().collect();
        for (left, right, _) in initializer.weighted_edges.iter() {
            let defect_vertices: Vec<_> = [left, right]
                .into_iter()
                .filter(|vertex_index| !virtual_vertices.contains(&vertex_index))
                .cloned()
                .collect();
            let solver = dual_module_comb_basic_standard_syndrome(
                d,
                visualize_filename.clone(),
                defect_vertices.clone(),
                true,
                false,
            );
            assert_eq!(solver.offloaded, defect_vertices.len(), "all defects should be offloaded");
        }
    }

    /// test layer fusion without any offloading
    #[test]
    fn dual_module_comb_layer_fusion_1() {
        // cargo test dual_module_comb_layer_fusion_1 -- --nocapture
        let visualize_filename = "dual_module_comb_layer_fusion_1.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, true);
    }

    /// test layer fusion and enable pre matching
    #[test]
    fn dual_module_comb_pre_matching_layer_fusion_1() {
        // cargo test dual_module_comb_pre_matching_layer_fusion_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_layer_fusion_1.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, true, true);
    }

    /// debug deadlock in comb layer fusion
    #[test]
    fn dual_module_comb_layer_fusion_debug_1() {
        // cargo test dual_module_comb_layer_fusion_debug_1 -- --nocapture
        let visualize_filename = "dual_module_comb_layer_fusion_debug_1.json".to_string();
        let defect_vertices = vec![1, 8, 9, 15, 20, 26];
        dual_module_comb_basic_standard_syndrome(5, visualize_filename, defect_vertices, false, true);
    }

    /// debug vertex 85 has negative grown value -500
    #[test]
    fn dual_module_comb_pre_matching_layer_fusion_debug_1() {
        // cargo test dual_module_comb_pre_matching_layer_fusion_debug_1 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_layer_fusion_debug_1.json".to_string();
        let defect_vertices = vec![56, 57, 74, 86];
        dual_module_comb_basic_standard_syndrome(11, visualize_filename, defect_vertices, true, true);
    }

    /// debug panic fusion-blossom/src/complete_graph.rs:197:45: no entry found for key
    /// defect_vertices = [1, 5, 24, 36, 45]
    #[test]
    fn dual_module_comb_pre_matching_layer_fusion_debug_2() {
        // cargo test dual_module_comb_pre_matching_layer_fusion_debug_2 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_layer_fusion_debug_2.json".to_string();
        crate::cli::execute_in_cli(
            [
                "",
                "benchmark",
                "3",
                "0.499",
                "--code-type",
                "circuit-level-planar-code",
                "--noisy-measurements",
                "3",
                "--verifier",
                "fusion-serial",
                "--enable-visualizer",
                "--visualizer-filename",
                visualize_filename.as_str(),
                "--print-syndrome-pattern",
                "--use-deterministic-seed",
                "--total-rounds",
                "154",
                "--starting-iteration",
                "153",
                "--primal-dual-type",
                "embedded-comb-pre-matching-layer-fusion",
            ],
            true,
        );
    }

    /// debug panic case
    /// defect_vertices = [163,212,213,223,224,265,273,281,323]
    #[test]
    fn dual_module_comb_pre_matching_layer_fusion_debug_3() {
        // cargo test dual_module_comb_pre_matching_layer_fusion_debug_3 -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_layer_fusion_debug_3.json".to_string();
        let defect_vertices = vec![163, 212, 213, 223, 224, 265, 273, 281, 323];
        println!("{defect_vertices:?}");
        let config = json!({
            "code_type": fusion_blossom::qecp::code_builder::CodeType::RotatedPlanarCode,
            "noise_model": fusion_blossom::qecp::noise_model_builder::NoiseModelBuilder::StimNoiseModel,
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": 7,
            "trim_isolated_vertices": false,
        });
        let mut code = QECPlaygroundCode::new(9, 0.005, config);
        let mut visualizer = Some(
            Visualizer::new(
                option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
                code.get_positions(),
                true,
            )
            .unwrap(),
        );
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        code.set_defect_vertices(&defect_vertices);
        let syndrome = code.get_syndrome();
        let mut solver = stacker::grow(crate::util::MAX_NODE_NUM * 1024, || {
            // lock to avoid environment variable races when testing in parallel
            let lock = ENV_PARAMETER_LOCK.lock();
            let tmp_env_offloading = tmp_env::set_var("SUPPORT_OFFLOADING", "1");
            let tmp_env_layer_fusion = tmp_env::set_var("SUPPORT_LAYER_FUSION", "1");
            let micro_config = MicroBlossomSingle::new(&initializer, &code.get_positions());
            let solver = SolverDualComb::new_native(micro_config, json!({}));
            drop(tmp_env_offloading);
            drop(tmp_env_layer_fusion);
            drop(lock);
            Box::new(solver)
        });
        use fusion_blossom::mwpm_solver::*;
        solver.solve_visualizer(&syndrome, visualizer.as_mut());
        let subgraph = solver.subgraph_visualizer(visualizer.as_mut());
        let mut standard_solver = fusion_blossom::mwpm_solver::SolverSerial::new(&initializer);
        standard_solver.solve_visualizer(&syndrome, None);
        let standard_subgraph = standard_solver.subgraph_visualizer(None);
        let mut subgraph_builder = fusion_blossom::primal_module::SubGraphBuilder::new(&initializer);
        subgraph_builder.load_subgraph(&subgraph);
        let total_weight = subgraph_builder.total_weight();
        subgraph_builder.load_subgraph(&standard_subgraph);
        let standard_total_weight = subgraph_builder.total_weight();
        assert_eq!(total_weight, standard_total_weight);
    }

    /// verify that all single error can be decoded totally offline with layer fusion
    #[test]
    fn dual_module_comb_pre_matching_layer_fusion_all_single_error() {
        // cargo test dual_module_comb_pre_matching_layer_fusion_all_single_error -- --nocapture
        let visualize_filename = "dual_module_comb_pre_matching_layer_fusion_all_single_error.json".to_string();
        let d = 5;
        let code = CodeCapacityPlanarCode::new(d, 0.1, 500);
        let initializer = code.get_initializer();
        let virtual_vertices: BTreeSet<_> = initializer.virtual_vertices.iter().cloned().collect();
        for (left, right, _) in initializer.weighted_edges.iter() {
            let defect_vertices: Vec<_> = [left, right]
                .into_iter()
                .filter(|vertex_index| !virtual_vertices.contains(&vertex_index))
                .cloned()
                .collect();
            let solver =
                dual_module_comb_basic_standard_syndrome(d, visualize_filename.clone(), defect_vertices.clone(), true, true);
            assert_eq!(solver.offloaded, defect_vertices.len(), "all defects should be offloaded");
        }
    }

    pub fn dual_module_comb_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
        support_offloading: bool,
        support_layer_fusion: bool,
    ) -> Box<SolverDualComb> {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer, positions| {
                // lock to avoid environment variable races when testing in parallel
                let lock = ENV_PARAMETER_LOCK.lock();
                let tmp_env_offloading = if support_offloading {
                    Some(tmp_env::set_var("SUPPORT_OFFLOADING", "1"))
                } else {
                    None
                };
                let tmp_env_layer_fusion = if support_layer_fusion {
                    Some(tmp_env::set_var("SUPPORT_LAYER_FUSION", "1"))
                } else {
                    None
                };
                let micro_config = MicroBlossomSingle::new(initializer, positions);
                let result = SolverDualComb::new_native(micro_config, json!({}));
                drop(tmp_env_offloading);
                drop(tmp_env_layer_fusion);
                drop(lock);
                Box::new(result)
            },
        )
    }
}
