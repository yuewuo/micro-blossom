use crate::dual_module_rtl::*;
use crate::primal_module_embedded_adaptor::*;
use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::dual_module_serial::*;
use fusion_blossom::mwpm_solver::*;
use fusion_blossom::pointers::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::primal_module_serial::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;
use serde_json::json;

pub struct SolverDualRTL {
    dual_module: DualModuleRTLAdaptor,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

impl FusionVisualizer for SolverDualRTL {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverDualRTL {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleRTLAdaptor::new_empty(initializer),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverDualRTL {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, mut visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.primal_module.solve_step_callback(
            &self.interface_ptr,
            syndrome_pattern,
            &mut self.dual_module,
            |interface, dual_module, primal_module, group_max_update_length| {
                #[cfg(test)]
                println!("group_max_update_length: {:?}", group_max_update_length);
                assert!(
                    group_max_update_length.get_none_zero_growth().is_none(),
                    "dual RTL never reports this"
                );
                if dual_module.grown > 0 {
                    interface.notify_grown(dual_module.grown);
                }
                dual_module.grown = 0;
                let first_conflict = format!("{:?}", group_max_update_length.peek().unwrap());
                if let Some(visualizer) = visualizer.as_mut() {
                    visualizer
                        .snapshot_combined(
                            format!("resolve {first_conflict}"),
                            vec![interface, dual_module, primal_module],
                        )
                        .unwrap();
                }
            },
        );

        if let Some(visualizer) = visualizer.as_mut() {
            visualizer
                .snapshot_combined("solved".to_string(), vec![&self.interface_ptr, &self.dual_module, self])
                .unwrap();
        }
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverPrimalEmbedded {
    dual_module: DualModuleSerial,
    primal_module: PrimalModuleEmbeddedAdaptor,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

impl FusionVisualizer for SolverPrimalEmbedded {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverPrimalEmbedded {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleSerial::new_empty(initializer),
            primal_module: PrimalModuleEmbeddedAdaptor::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverPrimalEmbedded {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverEmbeddedRTL {
    dual_module: DualModuleStackless<DualDriverTracked<DualModuleRTL, MAX_NODE_NUM>>,
    primal_module: PrimalModuleEmbedded<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
    subgraph_builder: SubGraphBuilder,
    defect_nodes: Vec<VertexIndex>,
}

impl FusionVisualizer for SolverEmbeddedRTL {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let value = self.dual_module.driver.driver.snapshot(abbrev);
        // snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverEmbeddedRTL {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleStackless::new(DualDriverTracked::new(DualModuleRTL::new_empty(initializer))),
            primal_module: PrimalModuleEmbedded::new(),
            subgraph_builder: SubGraphBuilder::new(initializer),
            defect_nodes: vec![],
        }
    }
}

impl PrimalDualSolver for SolverEmbeddedRTL {
    fn clear(&mut self) {
        self.primal_module.reset();
        self.dual_module.reset();
        self.subgraph_builder.clear();
        self.defect_nodes.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, mut visualizer: Option<&mut Visualizer>) {
        assert!(syndrome_pattern.erasures.is_empty());
        assert!(syndrome_pattern.dynamic_weights.is_empty());
        assert!(self.defect_nodes.is_empty(), "must call `clear` between different runs");
        for (node_index, &defect_index) in syndrome_pattern.defect_vertices.iter().enumerate() {
            self.dual_module.add_defect(ni!(defect_index), ni!(node_index));
            self.defect_nodes.push(defect_index);
        }
        if let Some(visualizer) = visualizer.as_mut() {
            visualizer.snapshot_combined("syndrome".to_string(), vec![self]).unwrap();
        }
        let (mut obstacle, _) = self.dual_module.find_obstacle();
        while !obstacle.is_none() {
            // println!("obstacle: {obstacle:?}");
            debug_assert!(
                obstacle.is_obstacle(),
                "dual module should spontaneously process all finite growth"
            );
            if let Some(visualizer) = visualizer.as_mut() {
                visualizer.snapshot_combined(format!("{obstacle:?}"), vec![self]).unwrap();
            }
            self.primal_module.resolve(&mut self.dual_module, obstacle);
            (obstacle, _) = self.dual_module.find_obstacle();
        }
        if let Some(visualizer) = visualizer.as_mut() {
            visualizer.snapshot_combined("solved".to_string(), vec![self]).unwrap();
        }
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        // this perfect matching is not necessarily complete when some of the matchings are inside the dual module
        let mut perfect_matching = PerfectMatching::new();
        let interface_ptr = DualModuleInterfacePtr::new_empty();
        let belonging = interface_ptr.downgrade();
        self.primal_module
            .iterate_perfect_matching(|_, node_index, match_target, _link| {
                let node = DualNodePtr::new_value(DualNode {
                    index: node_index.get() as NodeIndex,
                    class: DualNodeClass::DefectVertex {
                        defect_index: self.defect_nodes[node_index.get() as usize],
                    },
                    grow_state: DualNodeGrowState::Stay,
                    parent_blossom: None,
                    dual_variable_cache: (0, 0),
                    belonging: belonging.clone(),
                });
                match match_target {
                    CompactMatchTarget::Peer(peer_index) => {
                        let peer = DualNodePtr::new_value(DualNode {
                            index: peer_index.get() as NodeIndex,
                            class: DualNodeClass::DefectVertex {
                                defect_index: self.defect_nodes[peer_index.get() as usize],
                            },
                            grow_state: DualNodeGrowState::Stay,
                            parent_blossom: None,
                            dual_variable_cache: (0, 0),
                            belonging: belonging.clone(),
                        });
                        perfect_matching.peer_matchings.push((node, peer));
                    }
                    CompactMatchTarget::VirtualVertex(virtual_index) => {
                        perfect_matching
                            .virtual_matchings
                            .push((node, virtual_index.get() as VertexIndex));
                    }
                }
            });
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined("perfect matching".to_string(), vec![self, &perfect_matching])
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![self, &perfect_matching, &VisualizeSubgraph::new(&subgraph)],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        // cannot adapt: both the primal and dual node don't know all the information
        self.subgraph_builder.total_weight()
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            // "dual": self.dual_module.generate_profiler_report(),
            // "primal": self.primal_module.generate_profiler_report(),
        })
    }
}
